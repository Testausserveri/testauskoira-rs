use poise::serenity_prelude as serenity;

use crate::{Context, Error};

/// Saa kutsu Testausserverin GitHub-organisaatioon
#[poise::command(slash_command)]
pub async fn github(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Linkki github organisaatioon:\n<https://testausserveri.fi/github>")
        .await?;
    Ok(())
}

/// Täytä jäsenhakemus liittyäksesi Testausserveri ry:n jäseneksi
#[poise::command(slash_command)]
pub async fn liity(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("https://testausserveri.fi/link/jasenhakemus")
        .await?;
    Ok(())
}

/// Get a users avatar
#[poise::command(slash_command)]
pub async fn avatar(
    ctx: Context<'_>,
    #[description = "The user whose avatar is requested"] user: serenity::User,
) -> Result<(), Error> {
    let face = if let Some(guild_id) = ctx.guild_id() {
        if let Ok(member) = guild_id.member(ctx, user.id).await {
            member.face()
        } else {
            user.face()
        }
    } else {
        user.face()
    };
    ctx.say(face).await?;
    Ok(())
}
