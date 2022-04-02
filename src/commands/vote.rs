use std::sync::Arc;

use serenity::{
    builder::EditMessage,
    http::Http,
    model::{
        interactions::{
            application_command::ApplicationCommandInteraction,
            message_component::{ButtonStyle, MessageComponentInteraction},
            InteractionApplicationCommandCallbackDataFlags,
            InteractionResponseType::DeferredUpdateMessage,
        },
        user::User,
    },
};

use crate::{
    extensions::*,
    models::{Vote, VoteEvent, VoteEventOption},
    Context, Database,
};

fn generate_vote_message(
    message: &mut EditMessage,
    vote: VoteEvent,
    votes: Vec<Vote>,
    mut vote_options: Vec<VoteEventOption>,
    author: &User,
) {
    vote_options.sort_unstable_by_key(|v| v.option_number);
    let mut desc_vote_options = Vec::new();
    for v in &vote_options {
        let mut k = v.option_value.clone();
        k.push_str(": ");
        k.push_str(
            &votes
                .iter()
                .filter(|k| k.option_number == v.option_number)
                .count()
                .to_string(),
        );
        desc_vote_options.push(k);
    }
    let cur_time = chrono::Local::now().naive_local();
    message.embed(|e| {
        e.title(format!("Äänestä: {}", &vote.title));
        e.author(|a| {
            a.name(author.tag());
            a.icon_url(author.face())
        });
        e.description(desc_vote_options.join("\n"));
        e.color(serenity::utils::Color::KERBAL);
        e.footer(|f| {
            f.text(format!(
                "Ends in: {} seconds",
                vote.duration as i64 - (cur_time - vote.start_time).num_seconds()
            ))
        })
    });
    message.components(|c| {
        c.create_action_row(|r| {
            for o in vote_options {
                r.create_button(|b| {
                    b.style(ButtonStyle::Primary);
                    b.label(o.option_value);
                    b.custom_id(format!("vote_{}", o.option_number))
                });
            }
            r
        })
    });
}

pub async fn update_vote(http: &Http, db: &Database, vote_id: i32) -> Result<(), anyhow::Error> {
    let cur_time = chrono::Local::now().naive_local();
    let vote_event = db.get_vote_event_from_id(vote_id)?;
    if (vote_event.duration as i32) < (cur_time - vote_event.start_time).num_seconds() as i32 {
        return end_vote(http, db, vote_event).await;
    }
    let options = db.get_options_by_vote_id(vote_id).unwrap();
    let votes = db.get_votes_by_vote_id(vote_id).unwrap();
    let author = http.get_user(vote_event.author_id).await.unwrap();
    let mut message = http
        .get_message(vote_event.channel_id, vote_event.message_id)
        .await?;
    message
        .edit(&http, |m| {
            generate_vote_message(m, vote_event, votes, options, &author);
            m
        })
        .await?;
    Ok(())
}

pub async fn update_all_votes(http: Arc<Http>, db: Arc<Database>) -> Result<(), anyhow::Error> {
    let votes = db.get_vote_ids()?;
    for vote in votes {
        update_vote(&http, &db, vote).await?;
    }
    Ok(())
}

pub async fn end_vote(http: &Http, db: &Database, vote: VoteEvent) -> Result<(), anyhow::Error> {
    let mut message = http.get_message(vote.channel_id, vote.message_id).await?;
    message
        .edit(&http, |m| {
            m.add_embed(|e| {
                e.description("Vote has ended!");
                e.color(serenity::utils::Color::FOOYOO)
            });
            m.components(|c| c)
        })
        .await?;
    db.purge_vote(vote.id)?;
    Ok(())
}

pub async fn user_vote(ctx: &Context, interaction: MessageComponentInteraction) {
    let db = ctx.get_db().await;
    let option = interaction
        .data
        .custom_id
        .as_str()
        .strip_prefix("vote_")
        .unwrap()
        .parse::<i32>()
        .unwrap();
    db.user_vote(interaction.message.id.0, interaction.user.id.0, option)
        .unwrap();
    let id = db
        .get_vote_id_from_message_id(interaction.message.id.0)
        .unwrap();
    update_vote(&ctx.http, &db, id).await.unwrap();
    interaction
        .create_interaction_response(&ctx.http, |r| r.kind(DeferredUpdateMessage))
        .await
        .unwrap();
}

pub async fn create_vote(ctx: &Context, interaction: ApplicationCommandInteraction) {
    let mut options = String::new();
    let mut title = String::new();
    let mut duration = 10;
    for command_option in &interaction.data.options {
        match command_option.name.as_str() {
            "options" => {
                options = command_option
                    .value
                    .as_ref()
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string();
            }
            "title" => {
                title = command_option
                    .value
                    .as_ref()
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string();
            }
            "duration" => {
                duration = command_option.value.as_ref().unwrap().as_i64().unwrap();
            }
            _ => {}
        }
    }
    title.truncate(255);
    let mut options = options
        .split(',')
        .map(|o| &o[0..std::cmp::min(o.len(), 32)])
        .collect::<Vec<&str>>();
    options.retain(|o| !o.trim().is_empty());
    if options.len() < 2 {
        interaction
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| {
                    d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL);
                    d.content("Try putting more than 1 option")
                })
            })
            .await
            .unwrap();
        return;
    }
    let mut vote_message = interaction
        .channel_id
        .send_message(&ctx.http, |m| m.content("Osallistu äänestykseen!"))
        .await
        .unwrap();
    let db = ctx.get_db().await;
    let vote_id = db
        .new_vote_event(
            vote_message.id.0,
            vote_message.channel_id.0,
            interaction.user.id.0,
            &title,
            duration as u32,
            options,
        )
        .unwrap();
    let vote_event = db
        .get_vote_event_from_message_id(vote_message.id.0)
        .unwrap();
    let vote_options = db.get_options_by_vote_id(vote_id).unwrap();
    let Ok(_) = vote_message.edit(&ctx.http, |m| {
        generate_vote_message(m, vote_event, Vec::new(), vote_options, &interaction.user);
        m
    }).await else {
        db.purge_vote(vote_id).unwrap();
        interaction
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| {
                    d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL);
                    d.content("Invalid request, perhaps included too many options")
                })
            }).await.unwrap();
            return vote_message.delete(&ctx.http).await.unwrap();
    };
    interaction
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| {
                d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL);
                d.content("Will do!")
            })
        })
        .await
        .unwrap();
}
