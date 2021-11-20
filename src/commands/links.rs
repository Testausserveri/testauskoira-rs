use twilight_model::gateway::payload::incoming::MessageCreate;
use twilight_http::Client;
use std::sync::Arc;

pub async fn github(msg: Box<MessageCreate>, http: Arc<Client>) {
    http.create_message(msg.channel_id)
        .content("Linkki github organisaatioon:\n<https://koira.testausserveri.fi/github/join>")
        .unwrap()
        .exec()
        .await
        .unwrap();
}
