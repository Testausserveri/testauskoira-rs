use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::extensions::*;
use crate::utils::winner_showcase::display_winner;
use crate::ShardManagerContainer;

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
    }

    Ok(())
}

#[command]
#[owners_only]
async fn award_ceremony(ctx: &Context, _msg: &Message) -> CommandResult {
    let db = ctx.get_db().await;
    display_winner(ctx.http.to_owned(), db.to_owned()).await;
    Ok(())
}
