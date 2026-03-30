use std::sync::Arc;

use chrono::Utc;
use poise::serenity_prelude::{Http, ReactionType};

use crate::{commands::giveaway::end_giveaway, database::Database};

pub async fn update_giveaways(http: Arc<Http>, db: impl AsRef<Database>) {
    let db = db.as_ref();
    let reaction_emoji = crate::config::CONFIG.giveaway_reaction_emoji;
    for g in db.get_ongoing_giveaways().await.unwrap().iter() {
        let end = g.end_time.and_utc();
        let now = Utc::now();
        if now > end {
            match end_giveaway(&http, db, g, ReactionType::from(reaction_emoji)).await {
                Ok(_) => {
                    db.set_giveaway_completed(g.id, true).await.unwrap();
                    info!("Ended giveaway #{} successfully", g.id);
                }
                Err(e) => error!("Failed to end giveaway #{}, reason: {}", g.id, e),
            }
        }
    }
}
