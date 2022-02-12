pub mod activity_award;
pub mod giveaway_updater;

use std::sync::Arc;

use clokwerk::{AsyncScheduler, Job, TimeUnits};
use serenity::http::Http;

use crate::{
    database::Database,
    events::{activity_award::display_winner, giveaway_updater::update_giveaways},
};

pub fn setup_schedulers(scheduler: &mut AsyncScheduler, http: Arc<Http>, db: Arc<Database>) {
    {
        let http_clone = http.clone();
        let db_clone = db.clone();
        scheduler.every(1.day()).at("00:00").run(move || {
            let inner_http_clone = http_clone.clone();
            let inner_db_clone = db_clone.clone();
            async move {
                display_winner(inner_http_clone, inner_db_clone, 1).await;
            }
        });
    }
    {
        let http_clone = http.clone();
        let db_clone = db.clone();
        scheduler.every(30.seconds()).run(move || {
            let inner_http_clone = http_clone.clone();
            let inner_db_clone = db_clone.clone();
            async move {
                update_giveaways(inner_http_clone, inner_db_clone).await;
            }
        });
    }
}
