pub mod commands;
pub mod database;
pub mod extensions;
pub mod utils;
pub mod voting;

#[macro_use]
extern crate tracing;
#[macro_use]
extern crate serde_derive;

use std::collections::HashSet;
use std::env;
use std::sync::Arc;

use clokwerk::{Scheduler, TimeUnits};
use commands::owner::*;
use database::Database;
use extensions::*;
use serenity::async_trait;
use serenity::client::bridge::gateway::{GatewayIntents, ShardManager};
use serenity::framework::standard::macros::group;
use serenity::framework::StandardFramework;
use serenity::http::Http;
use serenity::model::event::{MessageUpdateEvent, ResumedEvent};
use serenity::model::gateway::Ready;
use serenity::model::prelude::*;
use serenity::prelude::*;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::FmtSubscriber;
use utils::winner_showcase::*;

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

pub struct BlacklistRegexes {
    last_edited: std::time::SystemTime,
    regexvec: Vec<regex::Regex>,
}

impl TypeMapKey for BlacklistRegexes {
    type Value = Arc<Mutex<BlacklistRegexes>>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
        let guild_id: u64 = env::var("GUILD_ID")
            .expect("No GUILD_ID in .env")
            .parse()
            .expect("Invalid GUILD_ID provided");
        let guild_id = serenity::model::id::GuildId::from(guild_id);
        guild_id
            .set_application_commands(&ctx.http, |commands| {
                commands.create_application_command(|command| {
                    command
                        .name("github")
                        .description("Vastaanota kutsu Testausserverin GitHub-organisaatioon")
                });
                commands.create_application_command(|command| {
                    command
                        .name("⛔ Ilmianna viesti")
                        .kind(application_command::ApplicationCommandType::Message)
                })
            })
            .await
            .unwrap();
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::ApplicationCommand(ref a) => match a.data.name.as_ref() {
                "⛔ Ilmianna viesti" => voting::handle_report(&ctx, a.to_owned()).await,
                "github" => commands::links::github(&ctx, a.to_owned()).await,
                _ => info!("Ignoring unknown interaction: `{}`", &a.data.name),
            },
            Interaction::MessageComponent(_) => {
                voting::handle_vote_interaction(ctx, interaction).await;
            }
            _ => {}
        };
    }

    async fn message(&self, ctx: Context, msg: Message) {
        let db = ctx.get_db().await;
        db.increment_message_count(msg.author.id.as_u64())
            .await
            .ok();

        let words = match std::fs::read_to_string("blacklist.txt") {
            Ok(s) => s,
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        std::fs::File::create("blacklist.txt")
                            .expect("Unable to create blacklist.txt");
                    }
                    _ => panic!("Unable to access blacklist.txt"),
                }
                String::new()
            }
        };

        let mut data = ctx.data.write().await;
        let regexes = data.get_mut::<BlacklistRegexes>().unwrap();

        let last_edited = std::fs::metadata("blacklist.txt")
            .unwrap()
            .modified()
            .unwrap();

        if last_edited != regexes.lock().await.last_edited {
            info!("Generating new blacklist regexes");
            let mut new_vec = Vec::new();
            for w in words.lines() {
                if w.is_empty() {
                    continue;
                }
                if let Ok(r) = regex::Regex::new(w) {
                    new_vec.push(r);
                }
            }
            *regexes.lock().await = BlacklistRegexes {
                last_edited,
                regexvec: new_vec,
            };
        }

        for re in &regexes.lock().await.regexvec {
            if re.is_match(&msg.content) {
                msg.delete(&ctx.http).await.ok();
                break;
            }
        }
    }

    async fn message_update(
        &self,
        ctx: Context,
        _: Option<Message>,
        _: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        voting::handle_edit(&ctx, &event).await;
    }

    async fn message_delete(
        &self,
        ctx: Context,
        _: ChannelId,
        message_id: MessageId,
        _: Option<GuildId>,
    ) {
        voting::handle_delete(&ctx, message_id).await;
    }

    async fn guild_member_addition(&self, ctx: Context, _guild_id: GuildId, member: Member) {
        info!("{} joined", member.user);
        let member_role = env::var("MEMBER_ROLE_ID")
            .expect("member role id not found in $MEMBER_ROLE_ID")
            .parse::<u64>()
            .expect("Invalid member role id");
        member.clone().add_role(&ctx.http, member_role).await.ok();
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

#[group]
#[commands(quit, award_ceremony)]
struct General;

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to load .env file");

    FmtSubscriber::builder()
        .with_env_filter(EnvFilter::new("info,sqlx::query=error"))
        .init();

    let database = Database::new().await;

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let application_id = env::var("APPLICATION_ID")
        .expect("Expected an application id")
        .parse::<u64>()
        .expect("Invalid application id form");
    let http = Http::new_with_token(&token);

    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    let words = match std::fs::read_to_string("blacklist.txt") {
        Ok(s) => s,
        Err(e) => {
            match e.kind() {
                std::io::ErrorKind::NotFound => {
                    std::fs::File::create("blacklist.txt").expect("Unable to create blacklist.txt");
                }
                _ => panic!("Unable to access blacklist.txt"),
            }
            String::new()
        }
    };

    let mut regexvec = Vec::new();

    for w in words.lines() {
        if w.is_empty() {
            continue;
        }
        if let Ok(re) = regex::Regex::new(w) {
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

    let framework = StandardFramework::new()
        .configure(|c| c.owners(owners).prefix("!"))
        .group(&GENERAL_GROUP);

    let mut client = Client::builder(&token)
        .application_id(application_id)
        .framework(framework)
        .event_handler(Handler)
        .intents(
            GatewayIntents::non_privileged()
                | GatewayIntents::GUILD_MEMBERS
                | GatewayIntents::GUILD_PRESENCES,
        )
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        data.insert::<Database>(Arc::new(database));
        data.insert::<BlacklistRegexes>(Arc::new(Mutex::new(blacklist)));
    }

    let shard_manager = client.shard_manager.clone();

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut scheduler = Scheduler::with_tz(chrono::Local);

    let db = client.get_db().await;
    let http = client.cache_and_http.http.clone();

    scheduler.every(1.day()).at("23:59").run(move || {
        runtime.block_on(display_winner(http.to_owned(), db.to_owned()));
    });

    let thread_handle = scheduler.watch_thread(std::time::Duration::from_millis(5000));

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        thread_handle.stop();
        shard_manager.lock().await.shutdown_all().await;
    });

    if let Err(e) = client.start().await {
        error!("Client error: {}", e);
    }
}
