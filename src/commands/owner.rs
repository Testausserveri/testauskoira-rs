use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};

use crate::{extensions::*, utils::winner_showcase::display_winner, ShardManagerContainer};

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
async fn award_ceremony(ctx: &Context, _msg: &Message, mut args: Args) -> CommandResult {
    let offset = match args.single::<i32>() {
        Ok(a) => a,
        _ => 0,
    };
    let db = ctx.get_db().await;
    display_winner(ctx.http.to_owned(), db.to_owned(), offset).await;
    Ok(())
}
