use serenity::{
    model::{
        id::RoleId,
        interactions::{
            message_component::MessageComponentInteraction,
            InteractionApplicationCommandCallbackDataFlags,
        },
        prelude::application_command::ApplicationCommandInteraction,
    },
    prelude::Context,
};

pub async fn handle_interaction(ctx: &Context, intercation: ApplicationCommandInteraction) {
    let guild_id: u64 = std::env::var("GUILD_ID")
        .expect("NO GUILD_ID in .env")
        .parse()
        .unwrap();
    let mut guild_roles = ctx.http.get_guild_roles(guild_id).await.unwrap();
    let roles = match std::fs::read_to_string("self_service_roles.txt") {
        Ok(s) => s,
        Err(e) => {
            match e.kind() {
                std::io::ErrorKind::NotFound => {
                    std::fs::File::create("self_service_roles.txt")
                        .expect("Unable to create self_service_roles.txt");
                }
                _ => panic!("Unable to access self_service_roles.txt"),
            }
            String::new()
        }
    };
    let roles: Vec<u64> = roles
        .lines()
        .map(|l| l.trim().parse::<u64>().unwrap_or(0))
        .collect();
    guild_roles.retain(|r| roles.contains(&r.id.0));
    if guild_roles.is_empty() {
        intercation
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| {
                    d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL);
                    d.content("Valitettavasti yhtään roolia ei ole vielä saatavilla")
                })
            })
            .await
            .unwrap();
        return;
    }
    intercation
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| {
                d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL);
                d.content("Muokkaa omia roolejasi");
                d.components(|c| {
                    c.create_action_row(|r| {
                        r.create_select_menu(|m| {
                            m.custom_id("give_role_menu");
                            m.options(|os| {
                                for role in guild_roles {
                                    os.create_option(|o| {
                                        o.label(role.name);
                                        o.value(role.id)
                                    });
                                }
                                os
                            })
                        })
                    })
                })
            })
        })
        .await
        .unwrap();
}

pub async fn handle_menu_button(ctx: &Context, interaction: MessageComponentInteraction) {
    let mut member = interaction.member.as_ref().unwrap().clone();
    let new_role = interaction.data.values[0].parse::<u64>().unwrap();
    let content = if member.roles.contains(&RoleId(new_role)) {
        member.remove_role(&ctx.http, RoleId(new_role)).await.ok();
        format!("Rooli <@&{}> poistettu!", &new_role)
    } else {
        member.add_role(&ctx.http, RoleId(new_role)).await.ok();
        format!("Rooli <@&{}> lisätty!", &new_role)
    };
    interaction
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| {
                d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL);
                d.content(content)
            })
        })
        .await
        .unwrap()
}
