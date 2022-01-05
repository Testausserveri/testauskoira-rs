use serenity::{async_trait, client};

use crate::database::Database;

#[async_trait]
pub trait ClientContextExt {
    async fn get_db(&self) -> Database;
}

#[async_trait]
impl ClientContextExt for client::Context {
    async fn get_db(&self) -> Database {
        self.data.read().await.get::<Database>().unwrap().clone()
    }
}

#[async_trait]
impl ClientContextExt for client::Client {
    async fn get_db(&self) -> Database {
        self.data.read().await.get::<Database>().unwrap().clone()
    }
}
