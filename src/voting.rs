// FIXME: un-unwrap();

use serenity::{
    builder::EditMessage,
    model::interactions::{
        application_command::ApplicationCommandInteraction,
        message_component::ButtonStyle,
        InteractionApplicationCommandCallbackDataFlags,
        InteractionResponseType::{ChannelMessageWithSource, DeferredUpdateMessage},
    },
};

use crate::{
    env,
    extensions::*,
    models::{CouncilVoting, SuspectMessageEdit, VotingAction},
    Channel, Context, Interaction, Message, MessageId, MessageUpdateEvent, User,
};

async fn is_reported(ctx: &Context, message_id: u64) -> bool {
    let db = ctx.get_db().await;
    db.is_reported(message_id).await.unwrap_or(false)
}

fn filter_votes(id: i32, actions: Vec<VotingAction>) -> String {
    let mut actions = actions.iter()
        .filter(|x| x.vote_type == id)
        .map(|x| format!("\n<@{}>", x.voter_user_id))
        .collect::<String>();
    if actions.is_empty() {
        actions = "-".to_string()
    }
    actions
}

fn generate_moderation_message(
    message: &mut EditMessage,
    voting: CouncilVoting,
    edits: Vec<SuspectMessageEdit>,
    votes: Vec<VotingAction>,
) {
    let guild_id = env::var("GUILD_ID").expect("NO GUILD_ID in .env");
    let message_link = format!(
        "https://discord.com/channels/{}/{}/{}",
        guild_id, voting.suspect_message_channel_id, voting.suspect_message_id
    );
    let delete_voters = filter_votes(0,votes.clone());
    let silence_voters = filter_votes(1,votes.clone());
    let block_reporter_voters = filter_votes(2,votes.clone());
    message.embed(|e| {
        e.color(serenity::utils::Color::RED);
        e.title("Viestistä on tehty ilmoitus!");
        e.field("Arvojäseniä paikalla", voting.moderators_online, true);
        e.field(
            "Viestin kanava",
            format!("<#{}>", voting.suspect_message_channel_id),
            true,
        );
        e.field(
            "Viestin lähettänyt",
            format!("<@{}>", voting.suspect_id),
            true,
        );
        e.field(
            "Ilmoitusten tehnyt",
            format!("<@{}>", voting.reporter_id),
            true,
        );
        e.description(format!(
            "Viestin sisältö:\n```\n{}```",
            voting.suspect_message_content
        ));
        e.field(
            format!(
                "Poistamisen puolesta {}/{}",
                voting.delete_votes, voting.delete_votes_required
            ),
            delete_voters,
            true,
        );
        e.field(
            format!(
                "Hiljennyksen puolesta {}/{}",
                voting.silence_votes, voting.silence_votes_required
            ),
            silence_voters,
            true,
        );
        e.field(
            format!(
                "Ilmoittajan estämisen puolesta {}/{}",
                voting.block_reporter_votes, voting.block_reporter_votes_required
            ),
            block_reporter_voters,
            true,
        );
        e.footer(|f| {
            f.text(format!(
                "Viesti lähetetty: {}",
                voting.suspect_message_send_time
            ))
        })
    });
    for edit in &edits {
        if edit.new_content.is_empty() {
            message.add_embed(|e| {
                e.title("Viesti on poistettu");
                e.footer(|f| f.text(format!("Poiston ajankohta: {}", edit.edit_time)))
            });
            break;
        }
        message.add_embed(|e| {
            e.title("Viestiä on muokattu");
            e.description(format!("Uusi sisältö:\n```\n{}```", edit.new_content));
            e.footer(|f| f.text(format!("Muokkausajankohta: {}", edit.edit_time)))
        });
    }
    message.components(|c| {
        c.create_action_row(|r| {
            r.create_button(|b| {
                b.label("Poista viesti");
                b.style(ButtonStyle::Secondary);
                if voting.delete_votes == voting.delete_votes_required
                    || (!edits.is_empty() && edits.last().unwrap().new_content.is_empty())
                {
                    b.disabled(true);
                }
                b.custom_id("delete_button")
            });
            r.create_button(|b| {
                b.label("Hiljennä jäsen");
                b.style(ButtonStyle::Danger);
                if voting.silence_votes == voting.silence_votes_required {
                    b.disabled(true);
                }
                b.custom_id("ban_button")
            });
            r.create_button(|b| {
                b.label("Estä ilmoittaja");
                b.style(ButtonStyle::Danger);
                if voting.block_reporter_votes == voting.block_reporter_votes_required {
                    b.disabled(true);
                }
                b.custom_id("abuse_button")
            });
            if !message_link.is_empty() {
                r.create_button(|b| {
                    b.label("Näytä viesti");
                    b.style(ButtonStyle::Link);
                    b.url(message_link)
                });
            }
            r
        })
    });
}

async fn update_voting_message(ctx: &Context, voting_message_id: u64) {
    let moderation_channel_id: u64 = env::var("MOD_CHANNEL_ID")
        .expect("No MOD_CHANNEL_ID in .env")
        .parse()
        .expect("Invalid MOD_CHANNEL_ID provided");
    let db = ctx.get_db().await;
    let event = db.get_voting_event(voting_message_id).await.unwrap();
    let votes = db.get_voting_event_votes(voting_message_id).await.unwrap();
    let edits = db.get_voting_event_edits(voting_message_id).await.unwrap();
    let mut message = ctx
        .http
        .get_message(moderation_channel_id, voting_message_id)
        .await
        .unwrap();
    message
        .edit(&ctx.http, |m| {
            generate_moderation_message(m, event, edits, votes);
            m
        })
        .await
        .unwrap()
}

// This handles a message_changed event an checks for
// reported messages that are edited. It then updates the message on the
// moderation channel with the message's new content and the time of the edit.
//
// (Due to discord limitations the maximum number of logged edits is 9 after which they will no
// longer be logged)
// NOTE: This could become a problem in which case a workaround can be implemented
pub async fn handle_edit(ctx: &Context, event: &MessageUpdateEvent) {
    if !is_reported(ctx, event.id.0).await {
        return;
    }
    let db = ctx.get_db().await;
    let voting_event = db.get_voting_event_for_message(event.id.0).await.unwrap();
    db.add_edit_event(event.to_owned(), voting_event.vote_message_id)
        .await
        .unwrap();
    update_voting_message(ctx, voting_event.vote_message_id as u64).await;
}

// This handles the deletion of a message
// First it check whether the message is reported
// After that it proceeds accordingly.
// If a reported message is deleted the deletion time will be logged into the embed-chain
pub async fn handle_delete(ctx: &Context, message_id: MessageId) {
    if !is_reported(ctx, message_id.0).await {
        return;
    }
    let db = ctx.get_db().await;
    let voting_event = db.get_voting_event_for_message(message_id.0).await.unwrap();
    db.message_deleted(
        chrono::Local::now().naive_local(),
        voting_event.vote_message_id,
    )
    .await
    .unwrap();
    update_voting_message(ctx, voting_event.vote_message_id as u64).await;
}

// Handles an event where a message was reported using the "⛔ Ilmianna viesti" message command
// This sends an embed to the moderation channel, containing some information about the message
// and the reported
pub async fn handle_report(ctx: &Context, interaction: ApplicationCommandInteraction) {
    let no_reports_role_id: u64 = env::var("NO_REPORTS_ROLE_ID")
        .expect("Expected NO_REPORTS_ROLE_ID in .env")
        .parse()
        .expect("Invalid NO_REPORTS_ROLE_ID provided");

    let guild_id: u64 = env::var("GUILD_ID")
        .expect("Expected GUILD_ID in .env")
        .parse()
        .expect("Invalid GUILD_ID provided");

    let moderation_channel_id = env::var("MOD_CHANNEL_ID")
        .expect("MOD_CHANNEL_ID id expected")
        .parse::<u64>()
        .expect("Invalid MOD_CHANNEL_ID provided");

    if interaction
        .user
        .has_role(&ctx.http, guild_id, no_reports_role_id)
        .await
        .unwrap()
    {
        info!("Skipping blacklisted reporter {}", interaction.user.id.0);
        interaction
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| {
                    d.flags(
                        InteractionApplicationCommandCallbackDataFlags::EPHEMERAL
                        );
                    d.content("Sinut on hyllytetty ilmoitus-ominaisuuden väärinkäytöstä :rage:! Ilmoitustasi ei lähetetty.")
                });
                r.kind(ChannelMessageWithSource)
            })
        .await
        .unwrap();
        return;
    }

    let message = if is_moderator(&ctx, &interaction.user).await {
        format!(
            "Viesti on ilmiannettu arvojäsenten neuvostolle, <#{}>",
            moderation_channel_id
        )
    } else {
        "Viesti on ilmiannettu arvojäsenten neuvostolle".to_string()
    };

    interaction
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| {
                d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL);
                d.content(message)
            });
            r.kind(ChannelMessageWithSource)
        })
        .await
        .unwrap();
    let suspect_message = interaction.data.resolved.messages.values().next().unwrap();
    if is_reported(ctx, suspect_message.id.0).await {
        info!(
            "The message {} is already reported! Skipping...",
            suspect_message.id.0
        );
        return;
    }
    let mods_online = get_online_mod_count(ctx).await;
    let moderation_channel = ctx.http.get_channel(moderation_channel_id).await.unwrap();
    let suspect = suspect_message.author.clone();
    let voting_message = moderation_channel
        .id()
        .send_message(&ctx.http, |m| {
            m.embed(|e| e.title("Viestistä on tehty ilmoitus!"))
        })
        .await
        .unwrap();
    let db = ctx.get_db().await;
    db.new_reported_message(
        voting_message.id.0,
        suspect_message.to_owned(),
        interaction.user.id.0,
        mods_online as i32,
    )
    .await
    .unwrap();
    update_voting_message(ctx, voting_message.id.0).await;
    let message_link = suspect_message.link_ensured(&ctx.http).await;
    suspect
        .dm(&ctx.http, |m| {
            m.content("Viestistäsi on tehty ilmoitus moderaattoreille!");
            m.components(|c| {
                c.create_action_row(|r| {
                    r.create_button(|b| {
                        b.label("Näytä viesti");
                        b.style(ButtonStyle::Link);
                        b.url(message_link)
                    })
                })
            })
        })
        .await
        .unwrap();
}

// Get the amount of online members who have access to the moderation channel.
// This is done by comparing the members of the channel to the member that are currently present on
// the server.
async fn get_online_mod_count(ctx: &Context) -> usize {
    let channelid = env::var("MOD_CHANNEL_ID")
        .expect("MOD_CHANNEL_ID id expected")
        .parse::<u64>()
        .expect("Invalid mod role id");
    if let Channel::Guild(channel) = ctx.http.get_channel(channelid).await.unwrap() {
        let precenses = ctx.cache.guild(channel.guild_id).await.unwrap().presences;
        let mut members = channel.members(&ctx.cache).await.unwrap();
        members.retain(|m| precenses.contains_key(&m.user.id) && !m.user.bot);
        return members.len();
    };
    unreachable!()
}

// Check if the given user is a moderator or not, based on their access to the moderation channel
async fn is_moderator(ctx: &Context, user: &User) -> bool {
    let channelid = env::var("MOD_CHANNEL_ID")
        .expect("MOD_CHANNEL_ID id expected")
        .parse::<u64>()
        .expect("Invalid mod role id");
    if let Channel::Guild(channel) = ctx.http.get_channel(channelid).await.unwrap() {
        return channel
            .permissions_for_user(&ctx.cache, user)
            .await
            .unwrap()
            .read_messages();
    }
    unreachable!();
}

// The function to handle a vote-addition event for the "delete_button"
// This function adds the vote then checks whether the goal is reached
// and then acts accordingly, either by deleting the message and then updating
// the announcement on the moderation channel or just by updating the announcement
async fn add_delete_vote(ctx: &Context, voter: User, message: &mut Message) {
    let db = ctx.get_db().await;
    let event = db.get_voting_event(message.id.0).await.unwrap();
    if event.delete_votes == event.delete_votes_required {
        return;
    }
    if db
        .add_vote(event.vote_message_id, voter.id.0 as i64, 0)
        .await
        .unwrap()
        == 0
    {
        return;
    }
    let event = db.get_voting_event(message.id.0).await.unwrap();
    if event.delete_votes == event.delete_votes_required {
        let message = ctx
            .http
            .get_message(
                event.suspect_message_channel_id as u64,
                event.suspect_message_id as u64,
            )
            .await
            .unwrap();
        message.delete(&ctx.http).await.unwrap();
        db.message_deleted(chrono::Local::now().naive_local(), event.vote_message_id)
            .await
            .unwrap();
    }
    update_voting_message(ctx, event.vote_message_id as u64).await;
}

// The function to handle a vote-addition event for the "ban_button"
// This function adds the vote then checks whether the goal is reached
// and then acts accordingly, either by banning the member and then updating
// the announcement on the moderation channel or just by updating the announcement
//
// NOTE: The ban actually only applies the "silenced" role upon the user
async fn add_silence_vote(ctx: &Context, voter: User, message: &mut Message) {
    let db = ctx.get_db().await;
    let event = db.get_voting_event(message.id.0).await.unwrap();
    if event.silence_votes == event.silence_votes_required {
        return;
    }
    if db
        .add_vote(event.vote_message_id, voter.id.0 as i64, 1)
        .await
        .unwrap()
        == 0
    {
        return;
    }
    let event = db.get_voting_event(message.id.0).await.unwrap();
    if event.silence_votes == event.silence_votes_required {
        let guild_id: u64 = env::var("GUILD_ID")
            .expect("Expected GUILD_ID in .env")
            .parse()
            .expect("Invalid GUILD_ID provided");
        let silence_role: u64 = env::var("SILENCED_ROLE_ID")
            .expect("Expected SILENCED_ROLE_ID in .env")
            .parse()
            .expect("Invalid SILENCED_ROLE_ID provided");
        let mut member = ctx
            .http
            .get_member(guild_id, event.suspect_id as u64)
            .await
            .unwrap();
        member.add_role(&ctx.http, silence_role).await.unwrap();
    }
    update_voting_message(ctx, event.vote_message_id as u64).await;
}

// This function handles the press off the "abuse_button"
// If the vote-goal is reached, the user will be given a
// role that prevents them from further abusing the reporting feature
async fn add_abuse_vote(ctx: &Context, voter: User, message: &mut Message) {
    let db = ctx.get_db().await;
    let event = db.get_voting_event(message.id.0).await.unwrap();
    if event.block_reporter_votes == event.block_reporter_votes_required {
        return;
    }
    if db
        .add_vote(event.vote_message_id, voter.id.0 as i64, 2)
        .await
        .unwrap()
        == 0
    {
        return;
    }
    let event = db.get_voting_event(message.id.0).await.unwrap();
    if event.block_reporter_votes == event.block_reporter_votes_required {
        let guild_id: u64 = env::var("GUILD_ID")
            .expect("Expected GUILD_ID in .env")
            .parse()
            .expect("Invalid GUILD_ID provided");
        let abuse_role: u64 = env::var("NO_REPORTS_ROLE_ID")
            .expect("Expected NO_REPORTS_ROLE_ID in .env")
            .parse()
            .expect("Invalid NO_REPORTS_ROLE_ID provided");
        let mut member = ctx
            .http
            .get_member(guild_id, event.suspect_id as u64)
            .await
            .unwrap();
        member.add_role(&ctx.http, abuse_role).await.unwrap();
    }
    update_voting_message(ctx, event.vote_message_id as u64).await;
}

// This function handles the vote-interactions and the report interaction and
// calls the appropriate functions for them (logging stuff in the logs)
pub async fn handle_vote_interaction(ctx: Context, interaction: Interaction) {
    if let Interaction::MessageComponent(mut component) = interaction {
        match component.data.custom_id.as_str() {
            "delete_button" => {
                info!("Delete vote by {}", component.user.tag());
                add_delete_vote(&ctx, component.user.clone(), &mut component.message).await;
            }
            "ban_button" => {
                info!("Ban vote by {}", component.user.tag());
                add_silence_vote(&ctx, component.user.clone(), &mut component.message).await;
            }
            "abuse_button" => {
                info!("Abuse vote by {}", component.user.tag());
                add_abuse_vote(&ctx, component.user.clone(), &mut component.message).await;
            }
            _ => panic!("Unknown interaction: {}", component.data.custom_id),
        }
        component
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| d.content("Ilmiannettu"));
                r.kind(DeferredUpdateMessage)
            })
            .await
            .unwrap();
    }
}
