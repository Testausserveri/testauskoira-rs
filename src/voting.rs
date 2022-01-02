use serenity::builder::CreateEmbed;
use serenity::model::channel::EmbedField;
use serenity::model::interactions::application_command::ApplicationCommandInteraction;
use serenity::model::interactions::message_component::{ActionRowComponent, ButtonStyle};

use crate::{
    env, Channel, ChannelId, Context, Interaction, Message, MessageId, MessageUpdateEvent, User,
};

// Check whether the provided message is reported or not (if there is a report of the message on
// the moderation channel)
// If the message is reported: Returns Some(message)
// else: Returns None
async fn is_reported(ctx: &Context, message_id: u64, mod_id: u64) -> Option<Message> {
    let mod_channel_id = ChannelId(mod_id);
    let cur_id = ctx.cache.current_user_id().await;
    let mut messages_after = mod_channel_id
        .messages(&ctx.http, |r| r.after(MessageId(message_id)))
        .await
        .unwrap();
    messages_after.retain(|m| {
        m.author.id == cur_id
            && !m.embeds.is_empty()
            && m.embeds[0]
                .fields
                .iter()
                .find(|f| f.name.starts_with("Viestin id"))
                .unwrap()
                .value
                .parse::<u64>()
                .unwrap()
                == message_id
    });
    match messages_after.is_empty() {
        true => None,
        false => Some(messages_after[0].clone()),
    }
}

// This function add the NO_REPORTS_ROLE for the user provided
async fn prevent_further_reports(ctx: &Context, user_id: u64) {
    let no_reports_role_id: u64 = env::var("NO_REPORTS_ROLE_ID")
        .expect("Expected NO_REPORTS_ROLE_ID in .env")
        .parse()
        .expect("Invalid NO_REPORTS_ROLE_ID provided");

    let guild_id: u64 = env::var("GUILD_ID")
        .expect("Expected GUILD_ID in .env")
        .parse()
        .expect("Invalid GUILD_ID provided");

    let mut member = ctx.http.get_member(guild_id, user_id).await.unwrap();
    member.add_role(&ctx.http, no_reports_role_id).await.unwrap();
    info!("Added no-reports-role for {}", user_id);
}

// This handles a message_changed event an checks for
// reported messages that are edited. It then updates the message on the
// moderation channel with the message's new content and the time of the edit.
//
// (Due to discord limitations the maximum number of logged edits is 9 after which they will no
// longer be logged)
// NOTE: This could become a problem in which case a workaround can be implemented
pub async fn handle_edit(ctx: &Context, event: &MessageUpdateEvent) {
    let moderation_channel_id = env::var("MOD_CHANNEL_ID")
        .expect("MOD_CHANNEL_ID id expected")
        .parse::<u64>()
        .expect("Invalid mod channel id");
    let mut embed_message = match is_reported(ctx, event.id.0, moderation_channel_id).await {
        None => return,
        Some(m) => m,
    };
    embed_message
        .edit(&ctx.http, |m| {
            m.add_embed(|e| {
                e.color(serenity::utils::Color::ORANGE);
                e.title(format!(
                    "Viestiä muokattu {}",
                    event
                        .edited_timestamp
                        .unwrap()
                        .with_timezone(&chrono::Local)
                ));
                e.description(format!(
                    "Uusi sisältö:\n```\n{}```",
                    event.content.as_ref().unwrap()
                ))
            })
        })
        .await
        .unwrap();
}

// This handles the deletion of a message
// First it check whether the message is reported
// After that it proceeds accordingly.
// If a reported message is deleted the deletion time will be logged into the embed-chain
pub async fn handle_delete(ctx: &Context, message_id: MessageId) {
    let moderation_channel_id = env::var("MOD_CHANNEL_ID")
        .expect("MOD_CHANNEL_ID id expected")
        .parse::<u64>()
        .expect("Invalid mod channel id");
    let mut embed_message = match is_reported(ctx, message_id.0, moderation_channel_id).await {
        None => return,
        Some(m) => m,
    };
    embed_message
        .edit(&ctx.http, |m| {
            m.add_embed(|e| {
                e.color(serenity::utils::Color::RED);
                e.title("Viesti poistettu");
                e.description(format!("Poiston ajankohta {}", chrono::Local::now()))
            });
            m.components(|c| {
                c.create_action_row(|r| {
                    r.create_button(|b| {
                        b.label("Poista viesti");
                        b.style(ButtonStyle::Secondary);
                        b.disabled(true);
                        b.custom_id("delete_button")
                    });
                    r.create_button(|b| {
                        b.label("Erota jäsen");
                        b.style(ButtonStyle::Danger);
                        b.custom_id("ban_button")
                    });
                    r.create_button(|b| {
                        b.label("Estä ilmoittaja");
                        b.style(ButtonStyle::Danger);
                        b.custom_id("abuse_button")
                    })
                })
            })
        })
        .await
        .unwrap();
}

// Handles an event where a message was reported using the "⛔ Ilmianna viesti" message command
// This sends an embed to the moderation channel, containing some information about the message
// and the reported
pub async fn handle_report(ctx: &Context, interaction: ApplicationCommandInteraction) {
    let no_reports_role_id: u64 = env::var("NO_REPORTS_ROLE_ID")
        .expect("Expected NO_REPORTS_ROLE_ID in .env")
        .parse()
        .expect("Invalid NO_REPORTS_ROLE_ID provided");

    let guild_id: u64 = env::var("GUILD_ID")
        .expect("Expected GUILD_ID in .env")
        .parse()
        .expect("Invalid GUILD_ID provided");

    if interaction.user.has_role(&ctx.http, guild_id, no_reports_role_id).await.unwrap() {
        info!("Skipping blacklisted reporter {}", interaction.user.id.0);
        interaction
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| {
                    d.flags(
                        serenity::model::interactions::InteractionApplicationCommandCallbackDataFlags::EPHEMERAL
                        );
                    d.content("Sinut on hyllytetty ilmoitus-ominaisuuden väärinkäytöstä :rage:! Ilmoitustasi ei lähetetty.")
                });
                r.kind(serenity::model::interactions::InteractionResponseType::ChannelMessageWithSource)
            })
        .await
        .unwrap();
        return;
    }
    interaction
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| {
                d.flags(
                    serenity::model::interactions::InteractionApplicationCommandCallbackDataFlags::EPHEMERAL
                    );
                d.content("Viesti on ilmiannettu arvojäsenten neuvostolle")
            });
            r.kind(serenity::model::interactions::InteractionResponseType::ChannelMessageWithSource)
        })
        .await
        .unwrap();
    let suspect_message = interaction.data.resolved.messages.values().next().unwrap();
    let moderation_channel_id = env::var("MOD_CHANNEL_ID")
        .expect("MOD_CHANNEL_ID id expected")
        .parse::<u64>()
        .expect("Invalid mod role id");
    if is_reported(ctx, suspect_message.id.0, moderation_channel_id)
        .await
        .is_some()
    {
        info!(
            "The message {} is already reported! Skipping...",
            suspect_message.id.0
        );
        return;
    }
    let mods_online = get_online_mod_count(ctx).await;
    let moderation_channel = ctx.http.get_channel(moderation_channel_id).await.unwrap();
    let suspect = suspect_message.author.clone();
    moderation_channel
        .id()
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.color(serenity::utils::Color::RED);
                e.title(format!(
                    "Käyttäjän {} viestistä on tehty ilmoitus!",
                    suspect.tag()
                ));
                e.field("Arvojäseniä paikalla", mods_online, true);
                e.field(
                    "Viestin kanava",
                    format!("<#{}>", suspect_message.channel_id.0),
                    true,
                );
                e.field("Viestin id", suspect_message.id, true);
                e.field("Käyttäjän id", suspect.id.0, true);
                e.field(
                    "Ilmoituksen tehnyt",
                    interaction.member.clone().unwrap().user,
                    true,
                );
                e.description(format!(
                    "Viestin sisältö:\n```\n{}```",
                    suspect_message.content
                ));
                e.field(
                    format!(
                        "Poistamisen puolesta 0/{}",
                        (mods_online as f32).sqrt().clamp(1., 3.).round()
                    ),
                    "-",
                    true,
                );
                e.field(
                    format!(
                        "Porttikiellon puolesta 0/{}",
                        (mods_online as f32).sqrt().round()
                    ),
                    "-",
                    true,
                );
                e.field(
                    format!(
                        "Ilmoittajan estämisen puolesta 0/{}",
                        (mods_online as f32).sqrt().clamp(1., 3.).round()
                    ),
                    "-",
                    true,
                );
                e.footer(|f| {
                    f.text(format!(
                        "Viesti lähetetty: {}",
                        suspect_message.timestamp.with_timezone(&chrono::Local)
                    ))
                })
            });
            m.components(|c| {
                c.create_action_row(|r| {
                    r.create_button(|b| {
                        b.label("Poista viesti");
                        b.style(ButtonStyle::Secondary);
                        b.custom_id("delete_button")
                    });
                    r.create_button(|b| {
                        b.label("Erota jäsen");
                        b.style(ButtonStyle::Danger);
                        b.custom_id("ban_button")
                    });
                    r.create_button(|b| {
                        b.label("Estä ilmoittaja");
                        b.style(ButtonStyle::Danger);
                        b.custom_id("abuse_button")
                    })
                })
            })
        })
        .await
        .unwrap();
    let message_link = suspect_message.link_ensured(&ctx.http).await;
    suspect
        .dm(&ctx.http, |m| {
            m.content("Viestistäsi on tehty ilmoitus moderaattoreille!");
            m.components(|c| {
                c.create_action_row(|r| {
                    r.create_button(|b| {
                        b.label("Näytä viesti");
                        b.style(ButtonStyle::Link);
                        b.url(message_link)
                    })
                })
            })
        })
        .await
        .unwrap();
}

// Get the amount of online members who have access to the moderation channel.
// This is done by comparing the members of the channel to the member that are currently present on
// the server.
async fn get_online_mod_count(ctx: &Context) -> usize {
    let channelid = env::var("MOD_CHANNEL_ID")
        .expect("MOD_CHANNEL_ID id expected")
        .parse::<u64>()
        .expect("Invalid mod role id");
    let channel = ctx.http.get_channel(channelid).await.unwrap();
    let channel = match channel {
        Channel::Guild(c) => c,
        _ => unreachable!(),
    };
    let precenses = ctx.cache.guild(channel.guild_id).await.unwrap().presences;
    let mut members = channel.members(&ctx.cache).await.unwrap();
    members.retain(|m| precenses.contains_key(&m.user.id) && !m.user.bot);
    members.len()
}

// The function to handle a vote-addition event for the "delete_button"
// This function adds the vote then checks whether the goal is reached
// and then acts accordingly, either by deleting the message and then updating
// the announcement on the moderation channel or just by updating the announcement
async fn add_delete_vote(ctx: &Context, voter: User, message: &mut Message) {
    // FIXME: Defeat the spaghettimonster
    let mut original_embed = message.embeds.first().unwrap().clone();
    if original_embed.title.as_ref().unwrap().contains("poistettu") {
        return;
    }
    let delete_field_index = original_embed
        .fields
        .iter()
        .position(|f| f.name.starts_with("Poistamisen"))
        .unwrap();
    if original_embed.fields[delete_field_index]
        .value
        .contains(&voter.id.to_string())
    {
        return;
    }
    let name = &original_embed.fields[delete_field_index].name;
    let mut current_count = name[name.rfind(' ').unwrap() + 1..name.rfind('/').unwrap()]
        .parse::<i64>()
        .unwrap();
    let required_count = name[name.rfind('/').unwrap() + 1..name.len()]
        .parse::<i64>()
        .unwrap();
    current_count += 1;
    if current_count >= required_count {
        let channel_mention = &original_embed
            .fields
            .iter()
            .find(|f| f.name.starts_with("Viestin kanava"))
            .unwrap()
            .value;
        let channel_id = channel_mention
            [channel_mention.find('#').unwrap() + 1..channel_mention.rfind('>').unwrap()]
            .parse::<u64>()
            .unwrap();
        let message_id = original_embed
            .fields
            .iter()
            .find(|f| f.name.starts_with("Viestin id"))
            .unwrap()
            .value
            .parse::<u64>()
            .unwrap();
        original_embed.title = Some(format!(
            "{} on poistettu!",
            &original_embed.title.as_ref().unwrap()[..original_embed
                .title
                .as_ref()
                .unwrap()
                .find("viesti")
                .unwrap()
                + 6]
        ));
        info!("Poistetaan viesti {} kanavalta {}", message_id, channel_id);
        let sus_message = ctx.http.get_message(channel_id, message_id).await.unwrap();
        sus_message.delete(&ctx.http).await.unwrap();
    }
    let new_name = format!("Poistamisen puolesta {}/{}", current_count, required_count);
    let new_value = match original_embed.fields[delete_field_index].value.as_ref() {
        "-" => format!("{}", voter),
        _ => format!(
            "{}\n{}",
            voter, &original_embed.fields[delete_field_index].value
        ),
    };
    original_embed.fields[delete_field_index] = EmbedField::new(new_name, new_value, false);
    let mut actionrow = message.components.clone();
    if let ActionRowComponent::Button(button) = &mut actionrow[0].components[0] {
        button.disabled = true;
    }
    let mut original_embeds = message.embeds.clone();
    original_embeds[0] = original_embed.clone();
    let mut original_embeds: Vec<CreateEmbed> = original_embeds
        .iter()
        .map(|e| CreateEmbed::from(e.to_owned()))
        .collect();
    if current_count >= required_count {
        original_embeds[0].footer(|f| {
            f.text(format!(
                "{}\nPoistettu: {}",
                original_embed.footer.clone().unwrap().text,
                chrono::Local::now()
            ))
        });
    }
    message
        .edit(&ctx.http, |m| {
            m.set_embeds(original_embeds);
            if current_count >= required_count {
                m.components(|c| {
                    c.create_action_row(|r| {
                        r.create_button(|b| {
                            b.label("Poista viesti");
                            b.style(ButtonStyle::Secondary);
                            b.disabled(true);
                            b.custom_id("delete_button")
                        });
                        r.create_button(|b| {
                            b.label("Erota jäsen");
                            b.style(ButtonStyle::Danger);
                            b.custom_id("ban_button")
                        });
                        r.create_button(|b| {
                            b.label("Estä ilmoittaja");
                            b.style(ButtonStyle::Danger);
                            b.custom_id("abuse_button")
                        })
                    })
                });
            }
            m
        })
        .await
        .unwrap();
}

// The function to handle a vote-addition event for the "ban_button"
// This function adds the vote then checks whether the goal is reached
// and then acts accordingly, either by banning the member and then updating
// the announcement on the moderation channel or just by updating the announcement
//
// NOTE: The ban applied on a member is a "soft-ban" which means the member will
// be banned (and their messages will be deleted from the past hour) and immediately
// after the unbanned
async fn add_ban_vote(ctx: &Context, voter: User, message: &mut Message) {
    let mut original_embed = message.embeds.first().unwrap().clone();
    if original_embed
        .title
        .as_ref()
        .unwrap()
        .contains("porttikielto")
    {
        return;
    }
    let ban_field_index = original_embed
        .fields
        .iter()
        .position(|f| f.name.starts_with("Porttikiellon"))
        .unwrap();
    if original_embed.fields[ban_field_index]
        .value
        .contains(&voter.id.to_string())
    {
        return;
    }
    let name = &original_embed.fields[ban_field_index].name;
    let mut current_count = name[name.rfind(' ').unwrap() + 1..name.rfind('/').unwrap()]
        .parse::<i64>()
        .unwrap();
    let required_count = name[name.rfind('/').unwrap() + 1..name.len()]
        .parse::<i64>()
        .unwrap();
    current_count += 1;
    if current_count >= required_count {
        let user_id = original_embed
            .fields
            .iter()
            .find(|f| f.name.starts_with("Käyttäjän id"))
            .unwrap()
            .value
            .parse::<u64>()
            .unwrap();
        let guild_id = env::var("GUILD_ID")
            .expect("GUILD_ID expected")
            .parse::<u64>()
            .expect("Invalid guild id");
        let member = ctx.http.get_member(guild_id, user_id).await.unwrap();
        info!("Annetaan porttikielto käyttäjälle {}", member.user);
        original_embed.title = Some(format!(
            "Käyttäjälle {} on annettu porttikielto!",
            member.user.tag()
        ));
        let member = ctx
            .http
            .get_member(guild_id, member.user.id.0)
            .await
            .unwrap();
        member
            .ban_with_reason(
                &ctx.http,
                1,
                format!(
                    "Voted off by the moderator council: {}",
                    message.link_ensured(&ctx.http).await
                ),
            )
            .await
            .unwrap();
        member.unban(&ctx.http).await.unwrap();
    }
    let new_name = format!(
        "Porttikiellon puolesta {}/{}",
        current_count, required_count
    );
    let new_value = match original_embed.fields[ban_field_index].value.as_ref() {
        "-" => format!("{}", voter),
        _ => format!(
            "{}\n{}",
            voter, &original_embed.fields[ban_field_index].value
        ),
    };
    original_embed.fields[ban_field_index] = EmbedField::new(new_name, new_value, false);
    let mut actionrow = message.components.clone();
    if let ActionRowComponent::Button(button) = &mut actionrow[0].components[0] {
        button.disabled = true;
    }
    let mut original_embeds = message.embeds.clone();
    original_embeds[0] = original_embed.clone();
    let mut original_embeds: Vec<CreateEmbed> = original_embeds
        .iter()
        .map(|e| CreateEmbed::from(e.to_owned()))
        .collect();
    if current_count >= required_count {
        original_embeds[0].footer(|f| {
            f.text(format!(
                "{}\nPorttikielto annettu: {}",
                original_embed.footer.clone().unwrap().text,
                chrono::Local::now()
            ))
        });
    }
    message
        .edit(&ctx.http, |m| {
            m.set_embeds(original_embeds);
            if current_count >= required_count {
                m.components(|c| {
                    c.create_action_row(|r| {
                        r.create_button(|b| {
                            b.label("Poista viesti");
                            b.style(ButtonStyle::Secondary);
                            b.disabled(true);
                            b.custom_id("delete_button")
                        });
                        r.create_button(|b| {
                            b.label("Erota jäsen");
                            b.style(ButtonStyle::Danger);
                            b.disabled(true);
                            b.custom_id("ban_button")
                        });
                        r.create_button(|b| {
                            b.label("Estä ilmoittaja");
                            b.style(ButtonStyle::Danger);
                            b.custom_id("abuse_button")
                        })
                    })
                });
            }
            m
        })
        .await
        .unwrap();
}

// This function handles the press off the "abuse_button"
// If the vote-goal is reached, the user will be given a
// role that prevents them from further abusing the reporting feature
async fn add_abuse_vote(ctx: &Context, voter: User, message: &mut Message) {
    let mut original_embed = message.embeds.first().unwrap().clone();
    let abuse_field_index = original_embed
        .fields
        .iter()
        .position(|f| f.name.starts_with("Ilmoittajan estämisen"))
        .unwrap();
    if original_embed.fields[abuse_field_index]
        .value
        .contains(&voter.id.to_string())
    {
        return;
    }
    let name = &original_embed.fields[abuse_field_index].name;
    let mut current_count = name[name.rfind(' ').unwrap() + 1..name.rfind('/').unwrap()]
        .parse::<i64>()
        .unwrap();
    let required_count = name[name.rfind('/').unwrap() + 1..name.len()]
        .parse::<i64>()
        .unwrap();
    current_count += 1;
    if current_count >= required_count {
        let user_id_string = &original_embed
            .fields
            .iter()
            .find(|f| f.name.starts_with("Ilmoituksen tehnyt"))
            .unwrap()
            .value;
        let user_id: u64 = user_id_string[user_id_string.find('@').unwrap()+1..user_id_string.rfind('>').unwrap()].parse().unwrap();
        original_embed.title = Some("Ilmoittaja on estetty!".to_string());
        prevent_further_reports(ctx, user_id).await;
    }
    let new_name = format!(
        "Ilmoittajan estämisen puolesta {}/{}",
        current_count, required_count
    );
    let new_value = match original_embed.fields[abuse_field_index].value.as_ref() {
        "-" => format!("{}", voter),
        _ => format!(
            "{}\n{}",
            voter, &original_embed.fields[abuse_field_index].value
        ),
    };
    original_embed.fields[abuse_field_index] = EmbedField::new(new_name, new_value, false);
    let mut actionrow = message.components.clone();
    if let ActionRowComponent::Button(button) = &mut actionrow[0].components[0] {
        button.disabled = true;
    }
    let mut original_embeds = message.embeds.clone();
    original_embeds[0] = original_embed.clone();
    let mut original_embeds: Vec<CreateEmbed> = original_embeds
        .iter()
        .map(|e| CreateEmbed::from(e.to_owned()))
        .collect();
    if current_count >= required_count {
        original_embeds[0].footer(|f| {
            f.text(format!(
                "{}\nIlmoittaja estetty: {}",
                original_embed.footer.clone().unwrap().text,
                chrono::Local::now()
            ))
        });
    }
    message
        .edit(&ctx.http, |m| {
            m.set_embeds(original_embeds);
            if current_count >= required_count {
                m.components(|c| {
                    c.create_action_row(|r| {
                        r.create_button(|b| {
                            b.label("Poista viesti");
                            b.style(ButtonStyle::Secondary);
                            b.custom_id("delete_button")
                        });
                        r.create_button(|b| {
                            b.label("Erota jäsen");
                            b.style(ButtonStyle::Danger);
                            b.custom_id("ban_button")
                        });
                        r.create_button(|b| {
                            b.label("Estä ilmoittaja");
                            b.disabled(true);
                            b.style(ButtonStyle::Danger);
                            b.custom_id("abuse_button")
                        })
                    })
                });
            }
            m
        })
        .await
        .unwrap();
}

// This function handles the vote-interactions and the report interaction and
// calls the appropriate functions for them (logging stuff in the logs)
pub async fn handle_vote_interaction(ctx: Context, interaction: Interaction) {
    if let Interaction::MessageComponent(mut component) = interaction {
        match component.data.custom_id.as_str() {
            "delete_button" => {
                info!("Delete vote by {}", component.user.tag());
                add_delete_vote(&ctx, component.user.clone(), &mut component.message).await;
            }
            "ban_button" => {
                info!("Ban vote by {}", component.user.tag());
                add_ban_vote(&ctx, component.user.clone(), &mut component.message).await;
            },
            "abuse_button" => {
                info!("Abuse vote by {}", component.user.tag());
                add_abuse_vote(&ctx, component.user.clone(), &mut component.message).await;
            }
            _ => panic!("Unknown interaction: {}", component.data.custom_id),
        }
        component
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| d.content("Ilmiannettu"));
                r.kind(
                    serenity::model::interactions::InteractionResponseType::DeferredUpdateMessage,
                )
            })
            .await
            .unwrap();
    }
}
