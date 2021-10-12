use crate::{database::Database};
use std::sync::Arc;
use serenity::{async_trait, client};

#[async_trait]
pub trait ClientContextExt {
    async fn get_db(&self) -> Arc<Database>;
}

#[async_trait]
impl ClientContextExt for client::Context {
    async fn get_db(&self) -> Arc<Database> {
        self.data.read().await.get::<Database>().unwrap().clone()
    }
}
