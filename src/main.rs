mod commands;
pub(crate) mod config;
mod database;
mod events;
mod models;
mod schema;
mod voting;

#[macro_use]
extern crate tracing;

use poise::serenity_prelude as serenity;
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};
use tokio::sync::Mutex;

use config::CONFIG;

use database::Database;
use voting::PendingEdits;

pub struct BlacklistRegexes {
    last_edited: std::time::SystemTime,
    regexvec: Vec<regex::Regex>,
}

pub struct Data {
    pub db: Arc<Database>,
    pub blacklist: Arc<Mutex<BlacklistRegexes>>,
    pub pending_edits: Arc<Mutex<PendingEdits>>,
    pub list_offsets: Arc<Mutex<HashMap<u64, i64>>>,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Message { new_message } => {
            handle_message(ctx, data, new_message).await;
        }
        serenity::FullEvent::MessageUpdate {
            event,
            old_if_available: _,
            new: _,
        } => {
            voting::handle_edit(ctx, data, event).await;

            if let Some(ref msg) = event.content {
                let regexes = data.blacklist.lock().await;
                for re in &regexes.regexvec {
                    if re.is_match(msg) {
                        ctx.http
                            .delete_message(event.channel_id, event.id, None)
                            .await
                            .ok();
                        return Ok(());
                    }
                }
            }
        }
        serenity::FullEvent::MessageDelete {
            deleted_message_id,
            channel_id: _,
            guild_id: _,
        } => {
            voting::handle_delete(ctx, data, *deleted_message_id).await;
        }
        serenity::FullEvent::GuildMemberUpdate {
            old_if_available,
            new,
            event: _,
        } => {
            if let (Some(old_member), Some(new_member)) = (old_if_available, new) {
                let silenced_role_id = serenity::RoleId::new(CONFIG.silenced_role_id);
                let old_silence = old_member.roles.contains(&silenced_role_id);
                let new_silence = new_member.roles.contains(&silenced_role_id);
                if new_silence && !old_silence {
                    info!("Silencing user: {}", &new_member.user);
                    data.db.silence_user(new_member.user.id.get()).await.ok();
                } else if old_silence && !new_silence {
                    info!("un-silencing user: {}", &new_member.user);
                    data.db.unsilence_user(new_member.user.id.get()).await.ok();
                }

                if old_member.pending && !new_member.pending {
                    if let Err(e) = new_member
                        .add_role(&ctx.http, serenity::RoleId::new(CONFIG.member_role_id))
                        .await
                    {
                        error!("Failed to add member role to {}: {}", new_member.user, e);
                    }
                }
            }
        }
        serenity::FullEvent::GuildMemberAddition { new_member } => {
            info!("{} joined", new_member.user);
            if let Ok(true) = data.db.is_silenced(new_member.user.id.get()).await {
                info!("Adding silenced role to user {}", new_member.user);
                if let Err(e) = new_member
                    .add_role(&ctx.http, serenity::RoleId::new(CONFIG.silenced_role_id))
                    .await
                {
                    error!("Failed to add silence role to {}: {}", new_member.user, e);
                }
            }
        }
        serenity::FullEvent::InteractionCreate {
            interaction: serenity::Interaction::Component(component),
        } => match component.data.custom_id.as_str() {
            "give_role_menu" => {
                commands::role::handle_menu_button(ctx, component.clone()).await;
            }
            "delete_button" | "ban_button" | "abuse_button" | "useless_button" => {
                voting::handle_vote_interaction(ctx, data, component.clone()).await;
            }
            id if id.starts_with("vote_") => {
                commands::vote::user_vote(ctx, data, component.clone()).await;
            }
            id if id.starts_with("GIVEAWAY_") => {
                commands::giveaway::handle_component_interaction(ctx, data, component.clone())
                    .await;
            }
            _ => {
                debug!(
                    "Unknown component interaction: {}",
                    component.data.custom_id
                );
            }
        },
        serenity::FullEvent::Resume { .. } => {
            info!("Resumed");
        }
        _ => {}
    }
    Ok(())
}

async fn handle_message(ctx: &serenity::Context, data: &Data, msg: &serenity::Message) {
    // Check and reload blacklist regexes if file changed, then check message
    {
        let last_edited = std::fs::metadata("blacklist.txt")
            .and_then(|m| m.modified())
            .ok();

        let mut regexes = data.blacklist.lock().await;
        if let Some(last_edited) = last_edited {
            if last_edited != regexes.last_edited {
                let words = match std::fs::read_to_string("blacklist.txt") {
                    Ok(s) => s,
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::NotFound {
                            std::fs::File::create("blacklist.txt")
                                .expect("Unable to create blacklist.txt");
                        } else {
                            panic!("Unable to access blacklist.txt");
                        }
                        String::new()
                    }
                };

                info!("Generating new blacklist regexes");
                let mut new_vec = Vec::new();
                for w in words.lines() {
                    if w.is_empty() {
                        continue;
                    }
                    if let Ok(r) = regex::RegexBuilder::new(w).case_insensitive(true).build() {
                        new_vec.push(r);
                    }
                }
                *regexes = BlacklistRegexes {
                    last_edited,
                    regexvec: new_vec,
                };
            }
        }

        for re in &regexes.regexvec {
            if re.is_match(&msg.content) {
                msg.delete(&ctx.http).await.ok();
                return;
            }
        }
    }

    // Increment message count
    if let Some(gid) = msg.guild_id {
        if gid == CONFIG.guild_id && !msg.author.bot {
            let is_private_thread = ctx
                .cache
                .guild(gid)
                .and_then(|g| {
                    g.channels
                        .get(&msg.channel_id)
                        .map(|c| c.kind == serenity::ChannelType::PrivateThread)
                })
                .unwrap_or(false);
            if !is_private_thread {
                data.db
                    .increment_message_count(&msg.author.id.get())
                    .await
                    .ok();
            }
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    LazyLock::force(&CONFIG);

    tracing_subscriber::fmt::init();

    let database = Arc::new(Database::new().await);
    let pending_edits = PendingEdits::new();

    let words = match std::fs::read_to_string("blacklist.txt") {
        Ok(s) => s,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                std::fs::File::create("blacklist.txt").expect("Unable to create blacklist.txt");
            } else {
                panic!("Unable to access blacklist.txt");
            }
            String::new()
        }
    };

    let mut regexvec = Vec::new();
    for w in words.lines() {
        if w.is_empty() {
            continue;
        }
        if let Ok(re) = regex::RegexBuilder::new(w).case_insensitive(true).build() {
            regexvec.push(re);
        } else {
            info!("Skipping invalid regex in `blacklist.txt`: {}", w);
        }
    }

    let blacklist = BlacklistRegexes {
        last_edited: std::fs::metadata("blacklist.txt")
            .unwrap()
            .modified()
            .unwrap(),
        regexvec,
    };

    let db_for_scheduler = database.clone();

    let data = Data {
        db: database,
        blacklist: Arc::new(Mutex::new(blacklist)),
        pending_edits: Arc::new(Mutex::new(pending_edits)),
        list_offsets: Arc::new(Mutex::new(HashMap::new())),
    };

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::links::github(),
                commands::links::liity(),
                commands::links::avatar(),
                commands::role::role(),
                commands::vote::vote(),
                commands::giveaway::giveaway(),
                commands::owner::quit(),
                commands::owner::award_ceremony(),
                voting::report_message(),
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("!".into()),
                ..Default::default()
            },
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_in_guild(
                    ctx,
                    &framework.options().commands,
                    serenity::GuildId::new(CONFIG.guild_id),
                )
                .await?;

                info!("Connected and commands registered");

                if let Some(id) = CONFIG.status_channel_id {
                    serenity::ChannelId::new(id)
                        .send_message(
                            &ctx.http,
                            serenity::CreateMessage::new().content(format!(
                                "Testauskoira on herännyt ja valmiina toimintaan! `{}`",
                                env!("GIT_HASH")
                            )),
                        )
                        .await
                        .unwrap();
                }

                // Setup schedulers
                let http = ctx.http.clone();
                let mut scheduler = clokwerk::AsyncScheduler::with_tz(chrono::Local);
                events::setup_schedulers(&mut scheduler, http, db_for_scheduler);

                tokio::spawn(async move {
                    loop {
                        scheduler.run_pending().await;
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                });

                Ok(data)
            })
        })
        .build();

    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::GUILD_MEMBERS
        | serenity::GatewayIntents::GUILD_PRESENCES
        | serenity::GatewayIntents::MESSAGE_CONTENT;

    let mut client = serenity::ClientBuilder::new(&CONFIG.discord_token, intents)
        .framework(framework)
        .await
        .expect("Err creating client");

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        std::process::exit(0);
    });

    if let Err(e) = client.start().await {
        error!("Client error: {}", e);
    }
}
