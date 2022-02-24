use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
    CacheAndHttp,
};

use crate::{events::activity_award::display_winner, extensions::*, Arc, ShardManagerContainer};

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
    let http = ctx.http.clone();
    let cache = ctx.cache.clone();
    let mut cache_and_http = CacheAndHttp::default();
    cache_and_http.http = http;
    cache_and_http.cache = cache;
    display_winner(Arc::new(cache_and_http), db, offset).await;
    Ok(())
}
