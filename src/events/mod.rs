pub mod activity_award;
pub mod giveaway_updater;

use std::sync::Arc;
use std::time::Duration;

use chrono::Local;
use poise::serenity_prelude::Http;

use crate::{
    database::Database,
    events::{activity_award::display_winner, giveaway_updater::update_giveaways},
};

fn duration_until_next_midnight() -> Duration {
    let now = Local::now();
    let tomorrow_midnight = (now.date_naive() + chrono::Days::new(1))
        .and_hms_opt(0, 0, 0)
        .expect("BUG: valid midnight time");
    let tomorrow_midnight = tomorrow_midnight
        .and_local_timezone(Local)
        .earliest()
        .expect("BUG: valid local midnight");
    let duration = tomorrow_midnight - now;
    duration.to_std().unwrap_or(Duration::from_secs(60))
}

pub fn spawn_schedulers(http: Arc<Http>, db: Arc<Database>) {
    // Daily activity award at 00:00 local time
    {
        let http = http.clone();
        let db = db.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(duration_until_next_midnight()).await;
                display_winner(http.clone(), db.clone(), 1).await;
            }
        });
    }

    // Giveaway updates every 30 seconds
    {
        let http = http.clone();
        let db = db.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                update_giveaways(http.clone(), db.clone()).await;
            }
        });
    }

    // Vote updates every 10 seconds
    {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(10)).await;
                if let Err(e) =
                    crate::commands::vote::update_all_votes(http.clone(), db.clone()).await
                {
                    error!("Error while updating votes: {}", e);
                }
            }
        });
    }
}
