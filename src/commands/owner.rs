use twilight_model::gateway::payload::incoming::MessageCreate;
use twilight_http::Client;
use std::sync::Arc;
use crate::{database::Database,utils::winner_showcase::*};

pub async fn award_ceremony(msg: Box<MessageCreate>, http: Arc<Client>, db: Arc<Database>) {
    display_winner(http, db).await;
}
