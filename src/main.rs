pub mod api;
pub mod commands;
pub mod database;
pub mod extensions;
pub mod utils;

#[macro_use]
extern crate tracing;
#[macro_use]
extern crate serde_derive;

use commands::{links::*, owner::*};
use database::Database;
use extensions::*;
use utils::winner_showcase::*;

use std::{collections::HashSet, env, sync::Arc};

use serenity::{
    async_trait,
    client::bridge::gateway::{GatewayIntents, ShardManager},
    framework::{standard::macros::group, StandardFramework},
    http::Http,
    model::prelude::*,
    model::{event::ResumedEvent, gateway::Ready},
    prelude::*,
};

use clokwerk::{Scheduler, TimeUnits};

use tracing_subscriber::FmtSubscriber;

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg: Message) {
        let db = ctx.get_db().await;
        db.increment_message_count(msg.author.id.as_u64())
            .await
            .ok();

        let words = std::fs::read_to_string("blacklist.txt")
            .expect("Expected blacklist.txt in running directory");
        for w in words.lines() {
            if msg.content.contains(w) {
                msg.delete(&ctx.http).await.ok();
            }
        }
    }

    async fn guild_member_addition(&self, ctx: Context, _guild_id: GuildId, member: Member) {
        info!("{} joined", member.user);
        let member_role = env::var("MEMBER_ROLE_ID")
            .expect("memer role id not found in $MEMBER_ROLE_ID")
            .parse::<u64>()
            .expect("Invalid member role id");
        member.clone().add_role(&ctx.http, member_role).await.ok();
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

#[group]
#[commands(quit, github, award_ceremony)]
struct General;

#[actix_rt::main]
async fn main() {
    dotenv::dotenv().expect("Failed to load .env file");

    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to start the logger");

    let database = Database::new().await;

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let http = Http::new_with_token(&token);

    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    let framework = StandardFramework::new()
        .configure(|c| c.owners(owners).prefix("!"))
        .group(&GENERAL_GROUP);

    let mut client = Client::builder(&token)
        .framework(framework)
        .event_handler(Handler)
        .intents(GatewayIntents::non_privileged() | GatewayIntents::GUILD_MEMBERS)
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        data.insert::<Database>(Arc::new(database));
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

    let server = api::webserver::start_api(client.cache_and_http.http.clone()).await;

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        thread_handle.stop();
        shard_manager.lock().await.shutdown_all().await;
        server.stop(true).await;
    });

    if let Err(why) = client.start().await {
        error!("Client error: {}", why);
    }
}
