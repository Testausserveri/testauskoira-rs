use std::sync::Arc;

use poise::serenity_prelude::{self as serenity, *};

use crate::{
    database::Database,
    models::{Vote, VoteEvent, VoteEventOption},
    Context, Data, Error,
};

fn generate_vote_message(
    vote: VoteEvent,
    votes: Vec<Vote>,
    mut vote_options: Vec<VoteEventOption>,
    author: &User,
) -> EditMessage {
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
    desc_vote_options.push(format!(
        "End time: <t:{}:R>",
        vote.start_time.and_utc().timestamp()
            + vote.duration as i64
    ));

    let embed = CreateEmbed::new()
        .title(format!("Äänestä: {}", &vote.title))
        .author(CreateEmbedAuthor::new(author.name.clone()).icon_url(author.face()))
        .description(desc_vote_options.join("\n"))
        .colour(Colour::KERBAL);

    let buttons: Vec<CreateButton> = vote_options
        .iter()
        .map(|o| {
            CreateButton::new(format!("vote_{}", o.option_number))
                .style(ButtonStyle::Primary)
                .label(&o.option_value)
        })
        .collect();

    EditMessage::new()
        .embed(embed)
        .components(vec![CreateActionRow::Buttons(buttons)])
}

pub async fn update_vote(
    http: &Http,
    db: &Database,
    vote_id: i32,
) -> Result<(), anyhow::Error> {
    let cur_time = chrono::Local::now().naive_local();
    let vote_event = db.get_vote_event_from_id(vote_id)?;
    if (vote_event.duration as i32) < (cur_time - vote_event.start_time).num_seconds() as i32 {
        return end_vote(http, db, vote_event).await;
    }
    let options = db.get_options_by_vote_id(vote_id).unwrap();
    let votes = db.get_votes_by_vote_id(vote_id).unwrap();
    let author = http.get_user(UserId::new(vote_event.author_id)).await.unwrap();
    let mut message = http
        .get_message(
            ChannelId::new(vote_event.channel_id),
            MessageId::new(vote_event.message_id),
        )
        .await?;
    let edit_msg = generate_vote_message(vote_event, votes, options, &author);
    message.edit(&http, edit_msg).await?;
    Ok(())
}

pub async fn update_all_votes(http: Arc<Http>, db: Arc<Database>) -> Result<(), anyhow::Error> {
    let votes = db.get_vote_ids()?;
    let cur_time = chrono::Local::now().naive_local();
    for vote in votes {
        let vote_event = db.get_vote_event_from_id(vote)?;
        if (vote_event.duration as i32) < (cur_time - vote_event.start_time).num_seconds() as i32 {
            end_vote(&http, &db, vote_event).await?;
        }
    }
    Ok(())
}

pub async fn end_vote(http: &Http, db: &Database, vote: VoteEvent) -> Result<(), anyhow::Error> {
    let mut message = http
        .get_message(
            ChannelId::new(vote.channel_id),
            MessageId::new(vote.message_id),
        )
        .await?;
    message
        .edit(
            &http,
            EditMessage::new()
                .embed(
                    CreateEmbed::new()
                        .description("Vote has concluded!")
                        .colour(Colour::FOOYOO),
                )
                .components(vec![]),
        )
        .await?;
    db.purge_vote(vote.id)?;
    Ok(())
}

pub async fn user_vote(
    ctx: &serenity::Context,
    data: &Data,
    interaction: ComponentInteraction,
) {
    let option = interaction
        .data
        .custom_id
        .as_str()
        .strip_prefix("vote_")
        .unwrap()
        .parse::<i32>()
        .unwrap();
    data.db
        .user_vote(
            interaction.message.id.get(),
            interaction.user.id.get(),
            option,
        )
        .unwrap();
    let id = data
        .db
        .get_vote_id_from_message_id(interaction.message.id.get())
        .unwrap();
    update_vote(&ctx.http, &data.db, id).await.unwrap();
    interaction
        .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
        .await
        .unwrap();
}

/// Aloita äänestys
#[poise::command(slash_command)]
pub async fn vote(
    ctx: Context<'_>,
    #[description = "Äänestyksen aihe"] title: String,
    #[description = "Äänestyksen vaihtoehdot, pilkulla erotettuina"] options: String,
    #[description = "Äänestyksen kesto sekunneissa"] duration: i64,
) -> Result<(), Error> {
    let mut title = title;
    title.truncate(255);
    let mut options = options
        .split(',')
        .map(|o| o.trim().chars().take(32).collect::<String>())
        .filter(|o| !o.is_empty())
        .collect::<Vec<_>>();
    options.dedup();
    if options.len() < 2 {
        ctx.send(
            poise::CreateReply::default()
                .ephemeral(true)
                .content("Try putting more than 1 unique option"),
        )
        .await?;
        return Ok(());
    }

    let serenity_ctx = ctx.serenity_context();
    let mut vote_message = ctx
        .channel_id()
        .send_message(
            &serenity_ctx.http,
            CreateMessage::new().content("Osallistu äänestykseen!"),
        )
        .await
        .unwrap();
    let data = ctx.data();
    let vote_id = data
        .db
        .new_vote_event(
            vote_message.id.get(),
            vote_message.channel_id.get(),
            ctx.author().id.get(),
            &title,
            duration as u32,
            options,
        )
        .unwrap();
    let vote_event = data
        .db
        .get_vote_event_from_message_id(vote_message.id.get())
        .unwrap();
    let vote_options = data.db.get_options_by_vote_id(vote_id).unwrap();
    let edit_msg =
        generate_vote_message(vote_event, Vec::new(), vote_options, ctx.author());
    if vote_message.edit(&serenity_ctx.http, edit_msg).await.is_err() {
        data.db.purge_vote(vote_id).unwrap();
        ctx.send(
            poise::CreateReply::default()
                .ephemeral(true)
                .content("Invalid request, perhaps included too many options"),
        )
        .await?;
        vote_message.delete(&serenity_ctx.http).await.unwrap();
        return Ok(());
    }
    ctx.send(
        poise::CreateReply::default()
            .ephemeral(true)
            .content("Will do!"),
    )
    .await?;
    Ok(())
}
