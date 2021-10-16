use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::ShardManagerContainer;
use crate::utils::winner_showcase::display_winner;
use crate::database::Database;

#[command]
#[owners_only]
async fn quit(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;

    if let Some(manager) = data.get::<ShardManagerContainer>() {
        msg.reply(ctx, "Shutting down").await?;
        manager.lock().await.shutdown_all().await;
    } else {
        msg.reply(ctx, "There was a problem getting the shard manager")
            .await?;

        return Ok(());
    }

    Ok(())
}

#[command]
#[owners_only]
async fn award_ceremony(ctx: &Context, _msg: &Message) -> CommandResult {
    let db = ctx.data.read().await.get::<Database>().unwrap().clone();
    display_winner(ctx.http.to_owned(), db.to_owned()).await;
    Ok(())
}
