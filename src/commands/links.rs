use serenity::{
    model::prelude::application_command::ApplicationCommandInteraction, prelude::Context,
};

pub async fn github(ctx: &Context, interaction: ApplicationCommandInteraction) {
    interaction
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| {
                d.content("Linkki github organisaatioon:\n<https://testausserveri.fi/github>")
            })
        })
        .await
        .unwrap();
}

pub async fn liity(ctx: &Context, interaction: ApplicationCommandInteraction) {
    interaction
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| {
                d.content("https://testausserveri.fi/link/jasenhakemus")
            })
        })
        .await
        .unwrap();
}
