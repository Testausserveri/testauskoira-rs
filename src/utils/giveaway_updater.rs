use crate::{
    commands::giveaway::{end_giveaway, GIVEAWAY_REACTION_CHAR},
    database::Database,
    Http, ReactionType,
};
use chrono::{DateTime, Utc};
use std::sync::Arc;

pub async fn update_giveaways(http: Arc<Http>, db: Database) {
    for g in db.get_uncompleted_giveaways().await.unwrap().iter() {
        let end = DateTime::<Utc>::from_utc(g.end_time, Utc);
        let now = Utc::now();
        if now > end {
            match end_giveaway(&http, &db, &g, ReactionType::from(GIVEAWAY_REACTION_CHAR)).await {
                Ok(_) => {
                    db.set_giveaway_completed(g.id, true).await.unwrap();
                    info!("Ended giveaway #{} successfully", g.id);
                }
                Err(e) => error!("Failed to end giveaway #{}, reason: {}", g.id, e),
            }
        }
    }
}
