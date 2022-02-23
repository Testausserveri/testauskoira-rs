use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::{commands::giveaway::end_giveaway, database::Database, Http, ReactionType};

pub async fn update_giveaways(http: Arc<Http>, db: impl AsRef<Database>) {
    let db = db.as_ref();
    let reaction_emoji: char = std::env::var("GIVEAWAY_REACTION_EMOJI")
        .unwrap_or("🎉".to_string())
        .parse()
        .expect("GIVEAWAY_REACTION_EMOJI is not a valid emoji");
    for g in db.get_ongoing_giveaways().await.unwrap().iter() {
        let end = DateTime::<Utc>::from_utc(g.end_time, Utc);
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
