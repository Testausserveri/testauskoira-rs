use chrono::{DateTime, Utc};
use rand::seq::SliceRandom;
use std::collections::HashMap;

use serenity::{
    builder::{CreateComponents, CreateEmbed, CreateEmbedFooter},
    model::{
        interactions::{
            message_component::ButtonStyle, InteractionApplicationCommandCallbackDataFlags,
            InteractionResponseType,
        },
        prelude::application_command::*,
    },
    prelude::*,
};

use crate::{
    database::Database, extensions::*, models::Giveaway, Http, Interaction, Message, ReactionType,
    User,
};

static DEFAULT_DURATION: i64 = 3600;
static DEFAULT_WINNERS: i64 = 1;
static DEFAULT_PRIZE: &str = "Nothing :(";
pub static GIVEAWAY_REACTION_CHAR: char = '🎉';

struct ListOffset;

impl TypeMapKey for ListOffset {
    type Value = HashMap<u64, i64>;
}

async fn ensure_offset_map(ctx: &Context) {
    let mut data = ctx.data.write().await;
    if !data.contains_key::<ListOffset>() {
        data.insert::<ListOffset>(HashMap::new());
    }
}

fn footer_with_text<S: Into<String>>(text: S) -> CreateEmbedFooter {
    let mut footer = CreateEmbedFooter {
        0: std::collections::HashMap::new(),
    };
    footer.text(text.into());
    footer
}

fn generate_list_components(offset: i64, giveaways: i64) -> CreateComponents {
    let mut c = CreateComponents { 0: Vec::new() };

    c.create_action_row(|r| {
        r.create_button(|b| {
            b.style(ButtonStyle::Secondary);
            b.custom_id("GIVEAWAY_list_back");
            b.label("Previous page");
            b.disabled(offset - 10 < 0)
        });
        r.create_button(|b| {
            b.style(ButtonStyle::Secondary);
            b.custom_id("GIVEAWAY_list_next");
            b.label("Next page");
            b.disabled(offset + 10 >= giveaways)
        })
    });
    c
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
        let mut e = CreateEmbed { 0: HashMap::new() };
        let winner_string = winners
            .iter()
            .map(|x| format!("<@{}>", x.user_id.to_string()))
            .collect::<Vec<String>>()
            .join(", ");

        e.title(format!("Giveaway #{}", g.id));
        e.description(format!(
            "**Prize**: {}\n**Winners**: {}\n**End time**: <t:{}:R>",
            g.prize,
            if g.completed {
                winner_string
            } else {
                format!("Max {}", g.max_winners)
            },
            g.end_time.timestamp()
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
                &http,
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

async fn roll_winners(candidates: &Vec<User>, max_winners: i64) -> Vec<User> {
    candidates
        .choose_multiple(&mut rand::thread_rng(), max_winners as usize)
        .map(|x| x.to_owned())
        .collect()
}

async fn reroll_giveaway(
    http: &Http,
    db: &Database,
    giveaway: &Giveaway,
    reaction: ReactionType,
) -> Result<(), anyhow::Error> {
    let mut message = http
        .get_message(giveaway.channel_id, giveaway.message_id)
        .await?;
    let candidates = get_reacters(&http, &message, reaction.clone())
        .await
        .map(|v| v.into_iter().filter(|x| !x.bot).collect::<Vec<User>>())?;
    let winners = roll_winners(&candidates, giveaway.max_winners).await;
    let winners_string = if winners.is_empty() {
        "Nobody...".to_string()
    } else {
        winners
            .iter()
            .map(|x| format!("<@{}>", x.id.0))
            .collect::<Vec<String>>()
            .join(", ")
    };

    message
        .edit(&http, |e| {
            e.embed(|e| {
                e.title(&giveaway.prize);
                e.description(format!("Winners: {}", winners_string));
                e.timestamp(DateTime::<Utc>::from_utc(giveaway.end_time, Utc));
                e.set_footer(footer_with_text(
                    format!("ID: {} | ended at", giveaway.id).as_str(),
                ))
            })
        })
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
    db.set_giveaway_winners(giveaway.id, &winners.iter().map(|u| u.id.0).collect())
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
    reroll_giveaway(&http, &db, giveaway, reaction.clone()).await?;
    db.end_giveaway(giveaway_id).await?;
    info!("Successfully ended giveaway {}", giveaway_id);
    Ok(())
}

pub async fn handle_component_interaction(ctx: &Context, interaction: &Interaction) {
    let db = ctx.get_db().await;
    ensure_offset_map(&ctx).await;
    let mut data = ctx.data.write().await;
    let offsets = data.get_mut::<ListOffset>().unwrap();

    if let Interaction::MessageComponent(component) = interaction.to_owned() {
        match component.data.custom_id.as_str() {
            "GIVEAWAY_list_back" => {
                let mut offset = offsets.get(&component.user.id.0).unwrap_or(&0).to_owned();
                offset -= 10;
                if offset < 0 {
                    offset = 0;
                }
                let giveaways = db.get_giveaways().await.unwrap().len() as i64;
                let embeds = generate_list_embeds(&db, offset).await;
                let components = generate_list_components(offset, giveaways);
                debug!(
                    "Showing previous 10 giveaways for user {}",
                    component.user.id.0
                );
                component
                    .create_interaction_response(&ctx.http, |r| {
                        r.kind(InteractionResponseType::UpdateMessage);
                        r.interaction_response_data(|d| {
                            d.set_components(components);
                            d.embeds(embeds)
                        })
                    })
                    .await
                    .unwrap();
                offsets.insert(component.user.id.0, offset);
            }
            "GIVEAWAY_list_next" => {
                let offset = offsets.get(&component.user.id.0).unwrap_or(&0).to_owned() + 10;
                let embeds = generate_list_embeds(&db, offset).await;
                let components = generate_list_components(offset, embeds.len() as i64);
                debug!(
                    "Showing next {} giveaways for user {}",
                    embeds.len(),
                    component.user.id.0
                );
                component
                    .create_interaction_response(&ctx.http, |r| {
                        r.kind(InteractionResponseType::UpdateMessage);
                        r.interaction_response_data(|d| {
                            d.set_components(components);
                            d.embeds(embeds)
                        })
                    })
                    .await
                    .unwrap();
                offsets.insert(component.user.id.0, offset);
            }
            _ => debug!("Unknown interaction: {}", component.data.custom_id),
        }
    }
}

pub async fn handle_interaction(ctx: &Context, interaction: ApplicationCommandInteraction) {
    let db = ctx.get_db().await;

    ensure_offset_map(&ctx).await;
    let mut data = ctx.data.write().await;
    let offset_map = data.get_mut::<ListOffset>().unwrap();

    let option = interaction
        .data
        .options
        .first()
        .expect("Giveaway subcommand missing");
    let sub_options = option.options.clone();

    match option.name.as_str() {
        "start" => {
            let channel = sub_options
                .by_name("channel")
                .expect("Missing channel option")
                .to_channel()
                .expect("Invalid channel option");
            let duration = sub_options
                .by_name("duration")
                .map_or(DEFAULT_DURATION, |x| x.to_i64().unwrap_or(DEFAULT_DURATION));
            let winners = sub_options
                .by_name("winners")
                .map_or(DEFAULT_WINNERS, |x| x.to_i64().unwrap_or(DEFAULT_WINNERS));
            let prize = sub_options
                .by_name("prize")
                .map_or(DEFAULT_PRIZE.to_string(), |x| {
                    x.to_string().unwrap_or(DEFAULT_PRIZE.to_string())
                });

            if winners < 1 || duration < 1 {
                interaction
                    .create_interaction_response(&ctx.http, |r| {
                        r.interaction_response_data(|d| {
                            d.content(
                                "Duration and winners must be positive integers or left empty.",
                            );
                            d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                        })
                    })
                    .await
                    .unwrap();
            } else {
                let now = chrono::Utc::now();
                let end = now + chrono::Duration::seconds(duration);
                let mut message = channel
                    .id
                    .send_message(&ctx.http, |c| {
                        c.content("@everyone");
                        c.embed(|e| {
                            e.title(&prize);
                            e.description(format!("{} winners", winners));
                            e.timestamp(end);
                            e.set_footer(footer_with_text("ID: ? | ends at"))
                        })
                    })
                    .await
                    .unwrap();

                message
                    .react(&ctx.http, ReactionType::from(GIVEAWAY_REACTION_CHAR))
                    .await
                    .unwrap();

                match db
                    .start_giveaway(&message, end.naive_utc(), winners, &prize)
                    .await
                {
                    Ok(id) => {
                        message
                            .edit(&ctx.http, |e| {
                                e.embed(|e| {
                                    e.title(prize);
                                    e.description(format!("{} winners", winners));
                                    e.timestamp(end);
                                    e.set_footer(footer_with_text(format!("ID: {} | ends at", id)))
                                })
                            })
                            .await
                            .unwrap();
                        interaction
                            .create_interaction_response(&ctx.http, |r| {
                                r.interaction_response_data(|d| {
                                    d.content(format!("Giveaway started in <#{}>", channel.id.0));
                                    d.flags(
                                        InteractionApplicationCommandCallbackDataFlags::EPHEMERAL,
                                    )
                                })
                            })
                            .await
                            .unwrap();
                        info!(
                            "Giveaway started by user {} in channel {}, id {}, duration {} seconds",
                            interaction.user.id.0, channel.id.0, id, duration
                        );
                    }
                    Err(e) => {
                        error!("Failed to start giveaway: {}", e);
                        message.delete(&ctx.http).await.unwrap();
                        interaction
                            .create_interaction_response(&ctx.http, |r| {
                                r.interaction_response_data(|d| {
                                    d.content("Giveaway failed to be started");
                                    d.flags(
                                        InteractionApplicationCommandCallbackDataFlags::EPHEMERAL,
                                    )
                                })
                            })
                            .await
                            .unwrap();
                    }
                }
            }
        }
        "list" => {
            let giveaways = db.get_giveaways().await.unwrap();
            let embeds = generate_list_embeds(&db, 0).await;
            offset_map.insert(interaction.user.id.0, 0);
            interaction
                .create_interaction_response(&ctx.http, |r| {
                    r.interaction_response_data(|d| {
                        d.embeds(embeds);
                        d.set_components(generate_list_components(0, giveaways.len() as i64));
                        d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                    })
                })
                .await
                .unwrap();
            info!("Showing list for user {}", interaction.user.id.0)
        }
        "reroll" => {
            let giveaway_id = sub_options
                .by_name("giveaway_id")
                .expect("Missing giveaway id")
                .to_i64()
                .expect("Invalid giveaway id");
            let giveaway = db.get_giveaway(giveaway_id).await.unwrap();

            reroll_giveaway(
                &ctx.http,
                &db,
                &giveaway,
                ReactionType::from(GIVEAWAY_REACTION_CHAR),
            )
            .await
            .unwrap();

            info!(
                "{} rerolled giveaway {}",
                interaction.user.id.0, giveaway_id
            );

            interaction
                .create_interaction_response(&ctx.http, |r| {
                    r.interaction_response_data(|d| {
                        d.content("Rerolled giveaway");
                        d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                    })
                })
                .await
                .unwrap();
        }
        "edit" => {
            let giveaway_id = sub_options
                .by_name("giveaway_id")
                .expect("Missing giveaway id option")
                .to_i64()
                .expect("Invalid giveaway id option");
            let field = sub_options
                .by_name("field")
                .expect("Missing field option")
                .to_string()
                .expect("Invalid field option");
            let new_value = sub_options
                .by_name("new_value")
                .expect("Missing new value option")
                .to_i64()
                .expect("Invalid new value option");

            let giveaway = db.get_giveaway(giveaway_id).await.unwrap();
            let mut message = ctx
                .http
                .get_message(giveaway.channel_id, giveaway.message_id)
                .await
                .unwrap();
            let footer = footer_with_text(format!("ID: {} | ends at", giveaway.id));

            match field.as_str() {
                "winners" => {
                    db.edit_giveaway_max_winners(giveaway_id, new_value)
                        .await
                        .unwrap();
                    message
                        .edit(&ctx.http, |e| {
                            e.embed(|e| {
                                e.title(giveaway.prize);
                                e.description(format!("{} winners", new_value));
                                e.timestamp(DateTime::<Utc>::from_utc(giveaway.end_time, Utc));
                                e.set_footer(footer)
                            })
                        })
                        .await
                        .unwrap();
                    interaction
                        .create_interaction_response(&ctx.http, |r| {
                            r.interaction_response_data(|d| {
                                d.content(format!("Max winners changed to {}", new_value));
                                d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                            })
                        })
                        .await
                        .unwrap();
                    info!(
                        "User {} changed giveaway {}'s max winners to {}",
                        interaction.user.id.0, giveaway_id, new_value
                    );
                }
                "duration" => {
                    let giveaway = db.get_giveaway(giveaway_id).await.unwrap();
                    let start_time = giveaway.start_time;
                    let new_time = start_time + chrono::Duration::seconds(new_value);

                    db.edit_giveaway_duration(giveaway_id, new_time)
                        .await
                        .unwrap();
                    message
                        .edit(&ctx.http, |e| {
                            e.embed(|e| {
                                e.title(giveaway.prize);
                                e.description(format!("{} winners", giveaway.max_winners));
                                e.timestamp(DateTime::<Utc>::from_utc(new_time, Utc));
                                e.set_footer(footer)
                            })
                        })
                        .await
                        .unwrap();
                    interaction
                        .create_interaction_response(&ctx.http, |r| {
                            r.interaction_response_data(|d| {
                                d.content(format!("Duration changed to {} seconds", new_value));
                                d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                            })
                        })
                        .await
                        .unwrap();
                    info!(
                        "User {} changed giveaway {}'s duration to {} seconds",
                        interaction.user.id.0, giveaway_id, new_value
                    );
                }
                _ => panic!("Attempt to edit unknown field {}", field),
            }
        }
        "end" => {
            let giveaway_id = sub_options
                .by_name("giveaway_id")
                .expect("Missing giveaway id option")
                .to_i64()
                .expect("Invalid giveaway id option");
            match db.get_giveaway(giveaway_id).await {
                Ok(giveaway) => {
                    if giveaway.completed {
                        interaction
                            .create_interaction_response(&ctx.http, |r| {
                                r.interaction_response_data(|d| {
                                    d.content("Giveaway has already ended");
                                    d.flags(
                                        InteractionApplicationCommandCallbackDataFlags::EPHEMERAL,
                                    )
                                })
                            })
                            .await
                            .unwrap();
                        return;
                    }

                    end_giveaway(
                        &ctx.http,
                        &db,
                        &giveaway,
                        ReactionType::from(GIVEAWAY_REACTION_CHAR),
                    )
                    .await
                    .unwrap();

                    info!(
                        "User {} manually ended giveaway {}",
                        interaction.user.id.0, giveaway_id
                    );

                    interaction
                        .create_interaction_response(&ctx.http, |r| {
                            r.interaction_response_data(|d| {
                                d.content(format!("Giveaway ended in <#{}>", giveaway.channel_id));
                                d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                            })
                        })
                        .await
                        .unwrap();
                }
                Err(e) => {
                    match e.downcast_ref::<diesel::result::Error>() {
                        Some(diesel::result::Error::NotFound) => {
                            interaction.create_interaction_response(&ctx.http, |r| {
                                r.interaction_response_data(|d| {
                                    d.content("Giveaway not found");
                                    d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                                })
                            }).await.unwrap();
                        }
                        _ => {
                            error!("Error while ending giveaway {}: {}", giveaway_id, e);
                            interaction.create_interaction_response(&ctx.http, |r| {
                                r.interaction_response_data(|d| {
                                    d.content("An error occurred while ending the giveaway");
                                    d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                                })
                            }).await.unwrap();
                        }
                    }
                }
            }
        }
        "delete" => {
            let giveaway_id = sub_options
                .by_name("giveaway_id")
                .expect("Missing giveaway id option")
                .to_i64()
                .expect("Invalid giveaway id option");

            match db.delete_giveaway(giveaway_id).await {
                Ok(giveaway) => {
                    let message = ctx
                        .http
                        .get_message(giveaway.channel_id, giveaway.message_id)
                        .await
                        .unwrap();
                    message.delete(&ctx.http).await.unwrap();

                    info!(
                        "User {} deleted giveaway {}",
                        interaction.user.id.0, giveaway_id
                    );

                    interaction
                        .create_interaction_response(&ctx.http, |r| {
                            r.interaction_response_data(|d| {
                                d.content("Giveaway deleted");
                                d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                            })
                        })
                        .await
                        .unwrap();
                }
                Err(e) => {
                    match e.downcast_ref::<diesel::result::Error>() {
                        Some(diesel::result::Error::NotFound) => {
                            interaction.create_interaction_response(&ctx.http, |r| {
                                r.interaction_response_data(|d| {
                                    d.content("Giveaway not found");
                                    d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                                })
                            }).await.unwrap();
                        }
                        _ => {
                            error!("Error while deleting giveaway {}: {}", giveaway_id, e);
                            interaction.create_interaction_response(&ctx.http, |r| {
                                r.interaction_response_data(|d| {
                                    d.content("An error occurred while deleting the giveaway");
                                    d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                                })
                            }).await.unwrap();
                        }
                    }
                }
            }
        }
        _ => panic!("Unknown command {}", interaction.data.name),
    }
}
