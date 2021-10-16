#![feature(async_closure)]
pub mod commands;
pub mod database;
pub mod extensions;
pub mod utils;

use commands::{owner::*, links::*};
use database::Database;
use utils::winner_showcase::*;

use std::{collections::HashSet, env, sync::Arc};

use serenity::{
    async_trait,
    client::bridge::gateway::{ShardManager, GatewayIntents},
    framework::{standard::macros::group, StandardFramework},
    http::Http,
    model::{event::ResumedEvent, gateway::Ready},
    prelude::*,
    model::prelude::*,
};

use clokwerk::{Scheduler, TimeUnits};

use tracing::{error, info};
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
        let db = ctx.data.read().await.get::<Database>().unwrap().clone();
        let words = std::fs::read_to_string("blacklist.txt").expect("Expected blacklist.txt in running directory");
        if let Ok(_) = db.increment_message_count(&msg.author.id.as_u64()).await {};
        for w in words.lines() {
            if msg.content.contains(w) {
                if let Ok(_) = msg.delete(&ctx.http).await {};
            }
        }
    }

    async fn guild_member_addition(&self, ctx: Context, _guild_id: GuildId, member: Member) {
        let member_name = member.guild_id.name(&ctx.cache).await.unwrap();
        info!("{} joined {}", member.user.name,member_name);
        //if let Ok(_) = member.clone().add_role(&ctx.http,728178268590571580).await {};
        member.clone().add_role(&ctx.http, 895943702151823380u64).await.unwrap();
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

#[group]
#[commands(quit, github)]
struct General;

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to load .env file");

    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to start the logger");

    let database = Database::new()
        .await;

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

    let db = client.data.read().await.get::<Database>().unwrap().clone();
    let http = client.cache_and_http.http.clone();

    scheduler.every(1.day()).at("23:59").run(move || {
        runtime.block_on(display_winner(http.to_owned(),db.to_owned()));
    });
    
    let thread_handle = scheduler.watch_thread(std::time::Duration::from_millis(10000));

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        thread_handle.stop();
        shard_manager.lock().await.shutdown_all().await;
    });

    if let Err(why) = client.start().await {
        error!("Client error: {}", why);
    }
}
