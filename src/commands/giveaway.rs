use poise::serenity_prelude::{self as serenity, *};
use rand::seq::SliceRandom;

use crate::{Context, Data, Error, database::Database, models::Giveaway};

#[derive(Debug, poise::ChoiceParameter)]
pub enum EditField {
    #[name = "Arpajaisten kesto"]
    Duration,
    #[name = "Arpajaisten voittajien lukumäärä"]
    Winners,
}

fn generate_list_components(offset: i64, giveaways: i64) -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new("GIVEAWAY_list_back")
            .style(ButtonStyle::Secondary)
            .label("Previous page")
            .disabled(offset - 10 < 0),
        CreateButton::new("GIVEAWAY_list_next")
            .style(ButtonStyle::Secondary)
            .label("Next page")
            .disabled(offset + 10 >= giveaways),
    ])]
}

async fn generate_list_embeds(db: &Database, offset: i64) -> Vec<CreateEmbed> {
    let giveaways = db.get_n_giveaways_with_offset(10, offset).await.unwrap();
    let mut giveaway_winners = Vec::with_capacity(giveaways.len());

    for g in giveaways.iter() {
        let winners = db.get_giveaway_winners(g.id).await.unwrap();
        giveaway_winners.push(winners);
    }

    let mut embeds: Vec<CreateEmbed> = Vec::new();
    for (g, winners) in giveaways.iter().zip(giveaway_winners.iter()) {
        let winner_string = winners
            .iter()
            .map(|x| format!("<@{}>", x.user_id))
            .collect::<Vec<String>>()
            .join(", ");

        let e = CreateEmbed::new()
            .title(format!("Giveaway #{}", g.id))
            .description(format!(
                "**Prize**: {}\n**Winners**: {}\n**End time**: <t:{}:R>",
                g.prize,
                if g.completed {
                    winner_string
                } else {
                    format!("Max {}", g.max_winners)
                },
                g.end_time.and_utc().timestamp()
            ));
        embeds.push(e);
    }
    embeds
}

async fn get_reacters(
    http: &Http,
    message: &Message,
    reaction: ReactionType,
) -> Result<Vec<User>, anyhow::Error> {
    let mut total_users: Vec<User> = Vec::new();
    loop {
        match message
            .reaction_users(
                http,
                reaction.clone(),
                Some(100),
                total_users.last().map(|x| x.id),
            )
            .await
        {
            Ok(mut users) => {
                if users.is_empty() {
                    return Ok(total_users);
                }
                total_users.append(&mut users);
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }
}

async fn roll_winners(candidates: &[User], max_winners: i64) -> Vec<User> {
    candidates
        .choose_multiple(&mut rand::thread_rng(), max_winners as usize)
        .map(|x| x.to_owned())
        .collect()
}

async fn roll_giveaway(
    http: &Http,
    db: &Database,
    giveaway: &Giveaway,
    reaction: ReactionType,
    excluded: Option<Vec<u64>>,
) -> Result<(), anyhow::Error> {
    let excluded = excluded.unwrap_or_default();
    let mut message = http
        .get_message(
            ChannelId::new(giveaway.channel_id),
            MessageId::new(giveaway.message_id),
        )
        .await?;
    let candidates = get_reacters(http, &message, reaction.clone())
        .await
        .map(|v| {
            v.into_iter()
                .filter(|x| !x.bot && !excluded.contains(&x.id.get()))
                .collect::<Vec<User>>()
        })?;
    let winners = roll_winners(&candidates, giveaway.max_winners).await;
    let winners_string = if winners.is_empty() {
        "Nobody...".to_string()
    } else {
        winners
            .iter()
            .map(|x| format!("<@{}>", x.id.get()))
            .collect::<Vec<String>>()
            .join(", ")
    };

    message
        .edit(
            &http,
            EditMessage::new().embed(
                CreateEmbed::new()
                    .title(&giveaway.prize)
                    .description(format!("Winners: {}", winners_string))
                    .timestamp(Timestamp::from(giveaway.end_time.and_utc()))
                    .footer(CreateEmbedFooter::new(format!(
                        "ID: {} | ended at",
                        giveaway.id
                    ))),
            ),
        )
        .await?;
    if winners.is_empty() {
        message
            .channel_id
            .say(
                &http,
                format!(":pensive: **Nobody** won **{}**...", giveaway.prize),
            )
            .await?;
    } else {
        message
            .channel_id
            .say(
                &http,
                format!(":tada: {} won **{}**!", winners_string, giveaway.prize),
            )
            .await?;
    }
    db.add_giveaway_winners(giveaway.id, &winners.iter().map(|u| u.id.get()).collect())
        .await?;
    info!("Successfully rolled winners for giveaway {}", giveaway.id);
    Ok(())
}

pub async fn end_giveaway(
    http: &Http,
    db: &Database,
    giveaway: &Giveaway,
    reaction: ReactionType,
) -> Result<(), anyhow::Error> {
    let giveaway_id = giveaway.id;
    roll_giveaway(http, db, giveaway, reaction, None).await?;
    db.end_giveaway(giveaway_id).await?;
    info!("Successfully ended giveaway {}", giveaway_id);
    Ok(())
}

pub async fn handle_component_interaction(
    ctx: &serenity::Context,
    data: &Data,
    component: ComponentInteraction,
) {
    let current_offset = {
        let offsets = data.list_offsets.lock().await;
        offsets.get(&component.user.id.get()).copied().unwrap_or(0)
    };

    let new_offset = match component.data.custom_id.as_str() {
        "GIVEAWAY_list_back" => (current_offset - 10).max(0),
        "GIVEAWAY_list_next" => current_offset + 10,
        _ => {
            debug!("Unknown interaction: {}", component.data.custom_id);
            return;
        }
    };

    let giveaways = data.db.get_giveaways().await.unwrap().len() as i64;
    let embeds = generate_list_embeds(&data.db, new_offset).await;
    let components = generate_list_components(new_offset, giveaways);

    debug!(
        "Showing giveaways at offset {} for user {}",
        new_offset,
        component.user.id.get()
    );

    component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embeds(embeds)
                    .components(components),
            ),
        )
        .await
        .unwrap();

    data.list_offsets
        .lock()
        .await
        .insert(component.user.id.get(), new_offset);
}

/// Luo arvonta tai hallitse käynnissä olevia arpajaisia
#[poise::command(
    slash_command,
    subcommands("start", "list", "reroll", "edit", "end", "delete"),
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn giveaway(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Luo ja aloita arvonta
#[poise::command(slash_command)]
pub async fn start(
    ctx: Context<'_>,
    #[description = "Arpajaisilmoituksen kanava"]
    #[channel_types("Text", "News")]
    channel: GuildChannel,
    #[description = "Arpajaisten kesto (sekunneissa)"] duration: Option<i64>,
    #[description = "Arpajaisten voittajien lukumäärä"] winners: Option<i64>,
    #[description = "Arpajaisten palkinto"] prize: Option<String>,
    #[description = "Rooli joka mainitaan arpajaisilmoituksessa"] mention: Option<Role>,
) -> Result<(), Error> {
    let data = ctx.data();
    let serenity_ctx = ctx.serenity_context();

    let giveaway_emoji = crate::config::CONFIG.giveaway_reaction_emoji;

    let duration = duration.unwrap_or(crate::config::CONFIG.giveaway_default_duration);
    let winners = winners.unwrap_or(crate::config::CONFIG.giveaway_default_winners);
    let prize = prize.unwrap_or_else(|| crate::config::CONFIG.giveaway_default_prize.clone());

    if winners < 1 || duration < 1 {
        ctx.send(
            poise::CreateReply::default()
                .ephemeral(true)
                .content("Duration and winners must be positive integers or left empty."),
        )
        .await?;
        return Ok(());
    }

    let now = chrono::Utc::now();
    let end = now + chrono::Duration::seconds(duration);
    let mut msg_builder = CreateMessage::new();
    if let Some(m) = mention {
        msg_builder = msg_builder.content(format!("<@&{}>", m.id.get()));
    }
    msg_builder = msg_builder.embed(
        CreateEmbed::new()
            .title(&prize)
            .description(format!("{} winners", winners))
            .timestamp(Timestamp::from(end))
            .footer(CreateEmbedFooter::new("ID: ? | ends at")),
    );

    let mut message = channel
        .id
        .send_message(&serenity_ctx.http, msg_builder)
        .await
        .unwrap();

    message
        .react(&serenity_ctx.http, ReactionType::from(giveaway_emoji))
        .await
        .unwrap();

    match data
        .db
        .start_giveaway(&message, end.naive_utc(), winners, &prize)
        .await
    {
        Ok(id) => {
            message
                .edit(
                    &serenity_ctx.http,
                    EditMessage::new().embed(
                        CreateEmbed::new()
                            .title(&prize)
                            .description(format!("{} winners", winners))
                            .timestamp(Timestamp::from(end))
                            .footer(CreateEmbedFooter::new(format!("ID: {} | ends at", id))),
                    ),
                )
                .await
                .unwrap();
            ctx.send(
                poise::CreateReply::default()
                    .ephemeral(true)
                    .content(format!("Giveaway started in <#{}>", channel.id.get())),
            )
            .await?;
            info!(
                "Giveaway started by user {} in channel {}, id {}, duration {} seconds",
                ctx.author().id.get(),
                channel.id.get(),
                id,
                duration
            );
        }
        Err(e) => {
            error!("Failed to start giveaway: {}", e);
            message.delete(&serenity_ctx.http).await.unwrap();
            ctx.send(
                poise::CreateReply::default()
                    .ephemeral(true)
                    .content("Giveaway failed to be started"),
            )
            .await?;
        }
    }
    Ok(())
}

/// Luetteloi arpajaiset
#[poise::command(slash_command)]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data();
    let giveaways = data.db.get_giveaways().await.unwrap();
    let embeds = generate_list_embeds(&data.db, 0).await;

    data.list_offsets
        .lock()
        .await
        .insert(ctx.author().id.get(), 0);

    let mut reply = poise::CreateReply::default()
        .ephemeral(true)
        .components(generate_list_components(0, giveaways.len() as i64));
    for embed in embeds {
        reply = reply.embed(embed);
    }
    ctx.send(reply).await?;
    info!("Showing list for user {}", ctx.author().id.get());
    Ok(())
}

/// Arvo uudelleen arpajaisten voittaja(t)
#[poise::command(slash_command)]
pub async fn reroll(
    ctx: Context<'_>,
    #[description = "Arvonnan tunniste"] giveaway_id: i64,
    #[description = "Salli entisten voittajien uudelleenvalitseminen, oletus = false"]
    allow_past: Option<bool>,
) -> Result<(), Error> {
    let data = ctx.data();
    let serenity_ctx = ctx.serenity_context();
    let allow_past = allow_past.unwrap_or(false);

    let giveaway_emoji = crate::config::CONFIG.giveaway_reaction_emoji;

    let giveaway = data.db.get_giveaway(giveaway_id).await.unwrap();

    let excluded = if !allow_past {
        Some(
            data.db
                .get_giveaway_winners(giveaway.id)
                .await
                .map(|x| x.into_iter().map(|x| x.user_id).collect())
                .unwrap(),
        )
    } else {
        None
    };

    if !allow_past {
        data.db
            .set_giveaway_winners_rerolled(giveaway_id, excluded.as_ref().unwrap())
            .await
            .unwrap();
    }

    roll_giveaway(
        &serenity_ctx.http,
        &data.db,
        &giveaway,
        ReactionType::from(giveaway_emoji),
        excluded,
    )
    .await
    .unwrap();

    info!(
        "{} rerolled giveaway {}",
        ctx.author().id.get(),
        giveaway_id
    );

    ctx.send(
        poise::CreateReply::default()
            .ephemeral(true)
            .content("Rerolled giveaway"),
    )
    .await?;
    Ok(())
}

/// Muokkaa arpajaisia
#[poise::command(slash_command)]
pub async fn edit(
    ctx: Context<'_>,
    #[description = "Arvonnan tunniste"] giveaway_id: i64,
    #[description = "Muokattava ominaisuus"] field: EditField,
    #[description = "Uusi arvo"] new_value: i64,
) -> Result<(), Error> {
    let data = ctx.data();
    let serenity_ctx = ctx.serenity_context();

    let giveaway = data.db.get_giveaway(giveaway_id).await.unwrap();
    let mut message = serenity_ctx
        .http
        .get_message(
            ChannelId::new(giveaway.channel_id),
            MessageId::new(giveaway.message_id),
        )
        .await
        .unwrap();

    match field {
        EditField::Winners => {
            data.db
                .edit_giveaway_max_winners(giveaway_id, new_value)
                .await
                .unwrap();
            message
                .edit(
                    &serenity_ctx.http,
                    EditMessage::new().embed(
                        CreateEmbed::new()
                            .title(&giveaway.prize)
                            .description(format!("{} winners", new_value))
                            .timestamp(Timestamp::from(giveaway.end_time.and_utc()))
                            .footer(CreateEmbedFooter::new(format!(
                                "ID: {} | ends at",
                                giveaway_id
                            ))),
                    ),
                )
                .await
                .unwrap();
            ctx.send(
                poise::CreateReply::default()
                    .ephemeral(true)
                    .content(format!("Max winners changed to {}", new_value)),
            )
            .await?;
            info!(
                "User {} changed giveaway {}'s max winners to {}",
                ctx.author().id.get(),
                giveaway_id,
                new_value
            );
        }
        EditField::Duration => {
            let start_time = giveaway.start_time;
            let new_time = start_time + chrono::Duration::seconds(new_value);

            data.db
                .edit_giveaway_duration(giveaway_id, new_time)
                .await
                .unwrap();
            message
                .edit(
                    &serenity_ctx.http,
                    EditMessage::new().embed(
                        CreateEmbed::new()
                            .title(&giveaway.prize)
                            .description(format!("{} winners", giveaway.max_winners))
                            .timestamp(Timestamp::from(new_time.and_utc()))
                            .footer(CreateEmbedFooter::new(format!(
                                "ID: {} | ends at",
                                giveaway.id
                            ))),
                    ),
                )
                .await
                .unwrap();
            ctx.send(
                poise::CreateReply::default()
                    .ephemeral(true)
                    .content(format!("Duration changed to {} seconds", new_value)),
            )
            .await?;
            info!(
                "User {} changed giveaway {}'s duration to {} seconds",
                ctx.author().id.get(),
                giveaway_id,
                new_value
            );
        }
    }
    Ok(())
}

/// Lopeta arpajaiset
#[poise::command(slash_command)]
pub async fn end(
    ctx: Context<'_>,
    #[description = "Arvonnan tunniste"] giveaway_id: i64,
) -> Result<(), Error> {
    let data = ctx.data();
    let serenity_ctx = ctx.serenity_context();

    let giveaway_emoji = crate::config::CONFIG.giveaway_reaction_emoji;

    match data.db.get_giveaway(giveaway_id).await {
        Ok(giveaway) => {
            if giveaway.completed {
                ctx.send(
                    poise::CreateReply::default()
                        .ephemeral(true)
                        .content("Giveaway has already ended"),
                )
                .await?;
                return Ok(());
            }

            end_giveaway(
                &serenity_ctx.http,
                &data.db,
                &giveaway,
                ReactionType::from(giveaway_emoji),
            )
            .await
            .unwrap();

            info!(
                "User {} manually ended giveaway {}",
                ctx.author().id.get(),
                giveaway_id
            );

            ctx.send(
                poise::CreateReply::default()
                    .ephemeral(true)
                    .content(format!("Giveaway ended in <#{}>", giveaway.channel_id)),
            )
            .await?;
        }
        Err(e) => match e.downcast_ref::<diesel::result::Error>() {
            Some(diesel::result::Error::NotFound) => {
                ctx.send(
                    poise::CreateReply::default()
                        .ephemeral(true)
                        .content("Giveaway not found"),
                )
                .await?;
            }
            _ => {
                error!("Error while ending giveaway {}: {}", giveaway_id, e);
                ctx.send(
                    poise::CreateReply::default()
                        .ephemeral(true)
                        .content("An error occurred while ending the giveaway"),
                )
                .await?;
            }
        },
    }
    Ok(())
}

/// Poista arpajaiset
#[poise::command(slash_command)]
pub async fn delete(
    ctx: Context<'_>,
    #[description = "Arvonnan tunniste"] giveaway_id: i64,
) -> Result<(), Error> {
    let data = ctx.data();
    let serenity_ctx = ctx.serenity_context();

    match data.db.delete_giveaway(giveaway_id).await {
        Ok(giveaway) => {
            let message = serenity_ctx
                .http
                .get_message(
                    ChannelId::new(giveaway.channel_id),
                    MessageId::new(giveaway.message_id),
                )
                .await
                .unwrap();
            message.delete(&serenity_ctx.http).await.unwrap();

            info!(
                "User {} deleted giveaway {}",
                ctx.author().id.get(),
                giveaway_id
            );

            ctx.send(
                poise::CreateReply::default()
                    .ephemeral(true)
                    .content("Giveaway deleted"),
            )
            .await?;
        }
        Err(e) => match e.downcast_ref::<diesel::result::Error>() {
            Some(diesel::result::Error::NotFound) => {
                ctx.send(
                    poise::CreateReply::default()
                        .ephemeral(true)
                        .content("Giveaway not found"),
                )
                .await?;
            }
            _ => {
                error!("Error while deleting giveaway {}: {}", giveaway_id, e);
                ctx.send(
                    poise::CreateReply::default()
                        .ephemeral(true)
                        .content("An error occurred while deleting the giveaway"),
                )
                .await?;
            }
        },
    }
    Ok(())
}
