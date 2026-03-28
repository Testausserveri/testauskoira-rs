// FIXME: un-unwrap();

use std::collections::HashSet;

use poise::serenity_prelude::{self as serenity, *};

use crate::{
    Data, Error,
    models::{CouncilVoting, SuspectMessageEdit, VotingAction},
};

pub struct PendingEdits {
    edits: HashSet<u64>,
}

impl PendingEdits {
    pub fn new() -> PendingEdits {
        Self {
            edits: HashSet::new(),
        }
    }

    pub fn add(&mut self, message_id: u64) {
        self.edits.insert(message_id);
    }

    pub fn remove(&mut self, message_id: u64) {
        self.edits.remove(&message_id);
    }

    pub fn contains(&self, message_id: u64) -> bool {
        self.edits.contains(&message_id)
    }
}

async fn is_reported(data: &Data, message_id: u64) -> bool {
    data.db.is_reported(message_id).await.unwrap_or(false)
}

fn filter_votes(id: i32, actions: &[VotingAction]) -> String {
    let mut actions = actions
        .iter()
        .filter(|x| x.vote_type == id)
        .map(|x| format!("\n<@{}>", x.voter_user_id))
        .collect::<String>();
    if actions.is_empty() {
        actions = "-".to_string()
    }
    actions
}

fn generate_moderation_message(
    voting: CouncilVoting,
    edits: Vec<SuspectMessageEdit>,
    votes: Vec<VotingAction>,
    suspect_tag: String,
) -> EditMessage {
    let message_link = format!(
        "https://discord.com/channels/{}/{}/{}",
        crate::config::CONFIG.guild_id,
        voting.suspect_message_channel_id,
        voting.suspect_message_id
    );
    let delete_voters = filter_votes(0, &votes);
    let silence_voters = filter_votes(1, &votes);
    let block_reporter_voters = filter_votes(2, &votes);

    let main_embed = CreateEmbed::new()
        .colour(Colour::RED)
        .title("Viestistä on tehty ilmoitus!")
        .field(
            "Arvojäseniä paikalla",
            format!("{}", voting.moderators_online),
            true,
        )
        .field(
            "Viestin kanava",
            format!("<#{}>", voting.suspect_message_channel_id),
            true,
        )
        .field(
            "Viestin lähettänyt",
            format!("<@{}>, {}", voting.suspect_id, suspect_tag),
            true,
        )
        .field(
            "Ilmoituksen tehnyt",
            format!("<@{}>", voting.reporter_id),
            true,
        )
        .description(format!(
            "Viestin sisältö:\n```\n{}```",
            voting.suspect_message_content
        ))
        .field(
            format!(
                "Poistamisen puolesta {}/{}",
                voting.delete_votes, voting.delete_votes_required
            ),
            &delete_voters,
            true,
        )
        .field(
            format!(
                "Hiljennyksen puolesta {}/{}",
                voting.silence_votes, voting.silence_votes_required
            ),
            &silence_voters,
            true,
        )
        .field(
            format!(
                "Ilmoittajan estämisen puolesta {}/{}",
                voting.block_reporter_votes, voting.block_reporter_votes_required
            ),
            &block_reporter_voters,
            true,
        )
        .footer(CreateEmbedFooter::new(format!(
            "Viesti lähetetty: {}",
            voting.suspect_message_send_time
        )));

    let mut embeds = vec![main_embed];
    for edit in &edits {
        if edit.new_content.is_empty() {
            embeds.push(CreateEmbed::new().title("Viesti on poistettu").footer(
                CreateEmbedFooter::new(format!("Poiston ajankohta: {}", edit.edit_time)),
            ));
            break;
        }
        embeds.push(
            CreateEmbed::new()
                .title("Viestiä on muokattu")
                .description(format!("Uusi sisältö:\n```\n{}```", edit.new_content))
                .footer(CreateEmbedFooter::new(format!(
                    "Muokkausajankohta: {}",
                    edit.edit_time
                ))),
        );
    }

    let mut delete_btn = CreateButton::new("delete_button")
        .label("Poista viesti")
        .style(ButtonStyle::Secondary);
    if voting.delete_votes == voting.delete_votes_required
        || (!edits.is_empty() && edits.last().unwrap().new_content.is_empty())
    {
        delete_btn = delete_btn.disabled(true);
    }

    let mut silence_btn = CreateButton::new("ban_button")
        .label("Hiljennä jäsen")
        .style(ButtonStyle::Danger);
    if voting.silence_votes == voting.silence_votes_required {
        silence_btn = silence_btn.disabled(true);
    }

    let link_btn = CreateButton::new_link(message_link).label("Näytä viesti");

    let mut abuse_btn = CreateButton::new("abuse_button")
        .label("Estä ilmoittaja")
        .style(ButtonStyle::Danger);
    if voting.block_reporter_votes == voting.block_reporter_votes_required {
        abuse_btn = abuse_btn.disabled(true);
    }

    let useless_btn = CreateButton::new("useless_button")
        .label(format!("{} klikkausta tuhlattu", voting.useless_clicks))
        .style(ButtonStyle::Success);

    let components = vec![
        CreateActionRow::Buttons(vec![delete_btn, silence_btn, link_btn, abuse_btn]),
        CreateActionRow::Buttons(vec![useless_btn]),
    ];

    EditMessage::new().embeds(embeds).components(components)
}

async fn update_voting_message(ctx: &serenity::Context, data: &Data, voting_message_id: u64) {
    let moderation_channel_id = crate::config::CONFIG.mod_channel_id;
    let event = data.db.get_voting_event(voting_message_id).await.unwrap();
    let votes = data
        .db
        .get_voting_event_votes(voting_message_id)
        .await
        .unwrap();
    let edits = data
        .db
        .get_voting_event_edits(voting_message_id)
        .await
        .unwrap();
    let mut message = ctx
        .http
        .get_message(
            ChannelId::new(moderation_channel_id),
            MessageId::new(voting_message_id),
        )
        .await
        .unwrap();
    let suspect_tag = if let Ok(user) = UserId::new(event.suspect_id).to_user(&ctx.http).await {
        user.name.clone()
    } else {
        String::from("[Poistettu käyttäjä]")
    };
    let edit_msg = generate_moderation_message(event, edits, votes, suspect_tag);
    message.edit(&ctx.http, edit_msg).await.unwrap()
}

/// This handles a message_changed event and checks for
/// reported messages that are edited.
pub async fn handle_edit(ctx: &serenity::Context, data: &Data, event: &MessageUpdateEvent) {
    if !is_reported(data, event.id.get()).await {
        return;
    }
    let voting_event = data
        .db
        .get_voting_event_for_message(event.id.get())
        .await
        .unwrap();
    data.db
        .add_edit_event(event.to_owned(), voting_event.vote_message_id)
        .await
        .unwrap();
    update_voting_message(ctx, data, voting_event.vote_message_id).await;
}

/// This handles the deletion of a message
pub async fn handle_delete(ctx: &serenity::Context, data: &Data, message_id: MessageId) {
    if !is_reported(data, message_id.get()).await {
        return;
    }
    let voting_event = data
        .db
        .get_voting_event_for_message(message_id.get())
        .await
        .unwrap();
    data.db
        .message_deleted(
            chrono::Local::now().naive_local(),
            voting_event.vote_message_id,
        )
        .await
        .unwrap();
    update_voting_message(ctx, data, voting_event.vote_message_id).await;
}

/// Handles an event where a message was reported using the context menu command
#[poise::command(context_menu_command = "\u{26d4} Ilmianna viesti")]
pub async fn report_message(ctx: crate::Context<'_>, msg: Message) -> Result<(), Error> {
    let data = ctx.data();
    let serenity_ctx = ctx.serenity_context();

    let no_reports_role_id = crate::config::CONFIG.no_reports_role_id;
    let guild_id = crate::config::CONFIG.guild_id;
    let moderation_channel_id = crate::config::CONFIG.mod_channel_id;

    if ctx
        .author()
        .has_role(
            &serenity_ctx.http,
            GuildId::new(guild_id),
            RoleId::new(no_reports_role_id),
        )
        .await
        .unwrap()
    {
        info!("Skipping blacklisted reporter {}", ctx.author().id.get());
        ctx.send(
            poise::CreateReply::default()
                .ephemeral(true)
                .content("Sinut on hyllytetty ilmoitus-ominaisuuden väärinkäytöstä :rage:! Ilmoitustasi ei lähetetty."),
        )
        .await?;
        return Ok(());
    }

    let response_message = if is_moderator(serenity_ctx, ctx.author()).await {
        format!(
            "Viesti on ilmiannettu arvojäsenten neuvostolle, <#{}>",
            moderation_channel_id
        )
    } else {
        "Viesti on ilmiannettu arvojäsenten neuvostolle".to_string()
    };

    ctx.send(
        poise::CreateReply::default()
            .ephemeral(true)
            .content(response_message),
    )
    .await?;

    if is_reported(data, msg.id.get()).await {
        info!(
            "The message {} is already reported! Skipping...",
            msg.id.get()
        );
        return Ok(());
    }
    let mods_online = get_online_mod_count(serenity_ctx).await;
    let moderation_channel = serenity_ctx
        .http
        .get_channel(ChannelId::new(moderation_channel_id))
        .await
        .unwrap();
    let voting_message = moderation_channel
        .id()
        .send_message(
            &serenity_ctx.http,
            CreateMessage::new().embed(CreateEmbed::new().title("Viestistä on tehty ilmoitus!")),
        )
        .await
        .unwrap();
    data.db
        .new_reported_message(
            voting_message.id.get(),
            msg,
            ctx.author().id.get(),
            mods_online as i32,
        )
        .await
        .unwrap();
    update_voting_message(serenity_ctx, data, voting_message.id.get()).await;
    Ok(())
}

/// Get the amount of online members who have access to the moderation channel.
async fn get_online_mod_count(ctx: &serenity::Context) -> usize {
    if let Channel::Guild(channel) = ctx
        .http
        .get_channel(ChannelId::new(crate::config::CONFIG.mod_channel_id))
        .await
        .unwrap()
    {
        let presences = ctx.cache.guild(channel.guild_id).unwrap().presences.clone();
        let mut members = channel.members(&ctx.cache).unwrap();
        members.retain(|m| presences.contains_key(&m.user.id) && !m.user.bot);
        return members.len();
    };
    unreachable!()
}

/// Check if the given user is a moderator or not
async fn is_moderator(ctx: &serenity::Context, user: &User) -> bool {
    if let Channel::Guild(channel) = ctx
        .http
        .get_channel(ChannelId::new(crate::config::CONFIG.mod_channel_id))
        .await
        .unwrap()
    {
        if let Ok(member) = ctx.http.get_member(channel.guild_id, user.id).await {
            if let Some(guild) = ctx.cache.guild(channel.guild_id) {
                return guild
                    .user_permissions_in(&channel, &member)
                    .read_message_history();
            }
        }
    }
    false
}

/// Handle the "delete_button" vote
async fn handle_delete_vote(ctx: &serenity::Context, data: &Data, voter: &User, message: &Message) {
    let event = data.db.get_voting_event(message.id.get()).await.unwrap();
    if event.delete_votes == event.delete_votes_required {
        return;
    }
    if data
        .db
        .add_vote(event.vote_message_id, voter.id.get(), 0)
        .await
        .unwrap()
        == 0
    {
        data.db
            .remove_vote(event.vote_message_id, voter.id.get(), 0)
            .await
            .unwrap();
    } else {
        let event = data.db.get_voting_event(message.id.get()).await.unwrap();
        if event.delete_votes == event.delete_votes_required {
            let suspect_msg = ctx
                .http
                .get_message(
                    ChannelId::new(event.suspect_message_channel_id),
                    MessageId::new(event.suspect_message_id),
                )
                .await
                .unwrap();
            suspect_msg.delete(&ctx.http).await.unwrap();
            data.db
                .message_deleted(chrono::Local::now().naive_local(), event.vote_message_id)
                .await
                .unwrap();
        }
    }
    update_voting_message(ctx, data, event.vote_message_id).await;
}

/// Handle the "ban_button" (silence) vote
async fn handle_silence_vote(
    ctx: &serenity::Context,
    data: &Data,
    voter: &User,
    message: &Message,
) {
    let event = data.db.get_voting_event(message.id.get()).await.unwrap();
    if event.silence_votes == event.silence_votes_required {
        return;
    }
    if data
        .db
        .add_vote(event.vote_message_id, voter.id.get(), 1)
        .await
        .unwrap()
        == 0
    {
        data.db
            .remove_vote(event.vote_message_id, voter.id.get(), 1)
            .await
            .unwrap();
    } else {
        let event = data.db.get_voting_event(message.id.get()).await.unwrap();
        if event.silence_votes == event.silence_votes_required {
            let mut member = ctx
                .http
                .get_member(
                    GuildId::new(crate::config::CONFIG.guild_id),
                    UserId::new(event.suspect_id),
                )
                .await
                .unwrap();
            data.db.silence_user(member.user.id.get()).await.ok();
            member
                .add_role(
                    &ctx.http,
                    RoleId::new(crate::config::CONFIG.silenced_role_id),
                )
                .await
                .ok();
            member
                .disable_communication_until_datetime(
                    &ctx.http,
                    Timestamp::from(chrono::Utc::now() + chrono::Duration::weeks(1)),
                )
                .await
                .unwrap();
            let rules_channel_id = crate::config::CONFIG.rules_channel_id;
            if member
                .user
                .dm(
                    &ctx.http,
                    CreateMessage::new().content(format!(
                        "Sinut on hiljennetty huonon käyttäytymisen vuoksi arvojäsenten toimesta.\n\nMikäli haluat keskusteluoikeutesi takaisin, voit olla yhteydessä Mastermindeihin joko yksityisviestitse tai sähköpostitse masterminds@testausserveri.fi. Tarkistathan sääntömme kanavalta <#{}>.",
                        rules_channel_id
                    )),
                )
                .await
                .is_err()
            {
                info!(
                    "Unable to send \"Silenced notification\" to {}",
                    member.user.id.get()
                );
            }
        }
    }
    update_voting_message(ctx, data, event.vote_message_id).await;
}

/// Handle the "abuse_button" vote
async fn handle_abuse_vote(ctx: &serenity::Context, data: &Data, voter: &User, message: &Message) {
    let event = data.db.get_voting_event(message.id.get()).await.unwrap();
    if event.block_reporter_votes == event.block_reporter_votes_required {
        return;
    }
    if data
        .db
        .add_vote(event.vote_message_id, voter.id.get(), 2)
        .await
        .unwrap()
        == 0
    {
        data.db
            .remove_vote(event.vote_message_id, voter.id.get(), 2)
            .await
            .unwrap();
    } else {
        let event = data.db.get_voting_event(message.id.get()).await.unwrap();
        if event.block_reporter_votes == event.block_reporter_votes_required {
            let member = ctx
                .http
                .get_member(
                    GuildId::new(crate::config::CONFIG.guild_id),
                    UserId::new(event.reporter_id),
                )
                .await
                .unwrap();
            member
                .add_role(
                    &ctx.http,
                    RoleId::new(crate::config::CONFIG.no_reports_role_id),
                )
                .await
                .unwrap();
        }
    }
    update_voting_message(ctx, data, event.vote_message_id).await;
}

async fn handle_useless_button(
    ctx: &serenity::Context,
    data: &Data,
    component: &ComponentInteraction,
) {
    data.db
        .add_useless_click(component.message.id.get())
        .await
        .unwrap();
    component
        .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
        .await
        .unwrap();
    let should_update = {
        let mut pending = data.pending_edits.lock().await;
        if !pending.contains(component.message.id.get()) {
            pending.add(component.message.id.get());
            true
        } else {
            false
        }
    };
    if should_update {
        update_voting_message(ctx, data, component.message.id.get()).await;
        data.pending_edits
            .lock()
            .await
            .remove(component.message.id.get());
    }
}

/// This function handles the vote-interactions and the report interaction
pub async fn handle_vote_interaction(
    ctx: &serenity::Context,
    data: &Data,
    component: ComponentInteraction,
) {
    match component.data.custom_id.as_str() {
        "delete_button" => {
            info!("Delete vote by {}", component.user.name);
            handle_delete_vote(ctx, data, &component.user, &component.message).await;
        }
        "ban_button" => {
            info!("Ban vote by {}", component.user.name);
            handle_silence_vote(ctx, data, &component.user, &component.message).await;
        }
        "abuse_button" => {
            info!("Abuse vote by {}", component.user.name);
            handle_abuse_vote(ctx, data, &component.user, &component.message).await;
        }
        "useless_button" => {
            handle_useless_button(ctx, data, &component).await;
            return;
        }
        _ => {
            debug!("Unknown interaction: {}", component.data.custom_id);
            return;
        }
    }
    component
        .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
        .await
        .unwrap();
}
