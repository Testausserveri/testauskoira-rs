pub mod webserver;
pub mod database;
pub mod commands;
pub mod utils;

use serde::ser::StdError;

#[macro_use]
extern crate tracing;
#[macro_use]
extern crate serde_derive;

use database::Database;

use commands::{links,owner};

use std::{env, sync::Arc, error::Error};

use clokwerk::{Scheduler, TimeUnits};

use futures::stream::StreamExt;
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::{cluster::{Cluster, ShardScheme}, Event};
use twilight_http::Client as HttpClient;
use twilight_model::gateway::Intents;

use tokio::runtime::Runtime;

use crate::utils::winner_showcase::display_winner;

#[actix_rt::main]
async fn main() -> Result<(), Box<(dyn StdError + Sync + std::marker::Send + 'static)>> {
    dotenv::dotenv().expect("Failed to load .env file");

    tracing_subscriber::fmt::init();

    let db = Arc::new(Database::new().await);

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let scheme = ShardScheme::Auto;
    let http = Arc::new(HttpClient::new(token.clone()));

    let (cluster, mut events) = Cluster::builder(token.to_owned(), Intents::GUILD_MESSAGES)
        .shard_scheme(scheme)
        .build()
        .await?;
    let cluster = Arc::new(cluster);

    let cluster_spawn = Arc::clone(&cluster);

    tokio::spawn(async move {
        cluster_spawn.up().await;
    });


    let server = webserver::start_api(
        http.clone(),
        db.clone(),
        )
        .await;

    let mut scheduler = Scheduler::with_tz(chrono::Local);

    let db_clone = db.to_owned();
    let http_clone = http.to_owned();

    let runtime = Arc::new(Runtime::new().unwrap());
    scheduler.every(1.day()).at("23:59").run(move || {
        runtime.block_on(display_winner(http_clone.clone(),db_clone.clone()));
    });

    let thread_handle = scheduler.watch_thread(std::time::Duration::from_millis(5000));

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        thread_handle.stop();
        server.stop(true).await;
    });

    let cache = InMemoryCache::builder()
        .resource_types(ResourceType::MESSAGE)
        .build();

    // FIXME: This creates an infinite loop.
    while let Some((shard_id, event)) = events.next().await {
        cache.update(&event);

        tokio::spawn(handle_event(shard_id, event, Arc::clone(&http),db.clone()));
    }
    Ok(())
}

async fn handle_event(
    shard_id: u64,
    event: Event,
    http: Arc<HttpClient>,
    db: Arc<Database>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match event {
        Event::MessageCreate(msg) => {
            db.increment_message_count(msg.0.author.id.get()).await.unwrap();
            match msg.content.as_str() {
                "!github" => links::github(msg,http).await,
                "!award_ceremony" => owner::award_ceremony(msg,http,db).await,
                _ => {},
            }
        }
        Event::Ready(ready) => {
        }
        Event::ShardConnected(_) => {
            println!("Connected on shard {}", shard_id);
        }
        _ => {}
    }

    Ok(())
}
