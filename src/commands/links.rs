use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

#[command]
async fn github(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Linkki github organisaatioon:\n<https://koira.testausserveri.fi/github/join>").await.unwrap();
    Ok(())
}
