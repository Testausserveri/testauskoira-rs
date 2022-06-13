use serenity::{
    model::{
        interactions::application_command::ApplicationCommandInteractionDataOptionValue,
        prelude::application_command::ApplicationCommandInteraction,
    },
    prelude::Context,
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

pub async fn avatar(ctx: &Context, interaction: ApplicationCommandInteraction) {
    let options = interaction.data.options.clone();
    for option in options {
        if let Some(ApplicationCommandInteractionDataOptionValue::User(u, pm)) = option.resolved {
            if let Some(m) = pm {
                if let Some(gid) = m.guild_id {
                    if let Ok(u) = ctx.http.get_member(gid.0, u.id.0).await {
                        return interaction
                            .create_interaction_response(&ctx.http, |r| {
                                r.interaction_response_data(|d| d.content(u.face()))
                            })
                            .await
                            .unwrap();
                    }
                }
            }
            return interaction
                .create_interaction_response(&ctx.http, |r| {
                    r.interaction_response_data(|d| d.content(u.face()))
                })
                .await
                .unwrap();
        }
    }
    interaction
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| d.content("Getting users avatar failed"))
        })
        .await
        .unwrap();
}
