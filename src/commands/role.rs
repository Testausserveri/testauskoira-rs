use poise::serenity_prelude::{self as serenity, *};

use crate::{Context, Error};

/// Valitse itsellesi mieluisia rooleja
#[poise::command(slash_command)]
pub async fn role(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id: u64 = std::env::var("GUILD_ID")
        .expect("NO GUILD_ID in .env")
        .parse()
        .unwrap();
    let mut guild_roles = ctx
        .serenity_context()
        .http
        .get_guild_roles(GuildId::new(guild_id))
        .await
        .unwrap();
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
    guild_roles.retain(|r| roles.contains(&r.id.get()));
    if guild_roles.is_empty() {
        ctx.send(
            poise::CreateReply::default()
                .ephemeral(true)
                .content("Valitettavasti yhtään roolia ei ole vielä saatavilla"),
        )
        .await?;
        return Ok(());
    }

    let options: Vec<CreateSelectMenuOption> = guild_roles
        .iter()
        .map(|role| CreateSelectMenuOption::new(&role.name, role.id.get().to_string()))
        .collect();

    let select_menu = CreateSelectMenu::new(
        "give_role_menu",
        CreateSelectMenuKind::String { options },
    );

    ctx.send(
        poise::CreateReply::default()
            .ephemeral(true)
            .content("Muokkaa omia roolejasi")
            .components(vec![CreateActionRow::SelectMenu(select_menu)]),
    )
    .await?;
    Ok(())
}

pub async fn handle_menu_button(
    ctx: &serenity::Context,
    interaction: ComponentInteraction,
) {
    let member = interaction.member.as_ref().unwrap().clone();
    let values = match &interaction.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => values,
        _ => return,
    };
    let new_role = values[0].parse::<u64>().unwrap();
    let new_role_id = RoleId::new(new_role);
    let content = if member.roles.contains(&new_role_id) {
        member.remove_role(&ctx.http, new_role_id).await.ok();
        format!("Rooli <@&{}> poistettu!", &new_role)
    } else {
        member.add_role(&ctx.http, new_role_id).await.ok();
        format!("Rooli <@&{}> lisätty!", &new_role)
    };
    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content(content),
            ),
        )
        .await
        .unwrap()
}
