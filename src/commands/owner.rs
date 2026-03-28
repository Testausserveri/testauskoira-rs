use crate::{Context, Error, events::activity_award::display_winner};

/// Shut down the bot
#[poise::command(prefix_command, owners_only)]
pub async fn quit(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Shutting down").await?;
    ctx.framework().shard_manager().shutdown_all().await;
    Ok(())
}

/// Manually trigger the daily award ceremony
#[poise::command(prefix_command, owners_only)]
pub async fn award_ceremony(
    ctx: Context<'_>,
    #[description = "Day offset"] offset: Option<i32>,
) -> Result<(), Error> {
    let offset = offset.unwrap_or(0);
    let db = ctx.data().db.clone();
    let http = ctx.serenity_context().http.clone();
    display_winner(http, db, offset).await;
    Ok(())
}
