use serenity::prelude::TypeMapKey;
pub mod giveaway;
pub mod message_logging;
pub mod voting;

use diesel::{
    mysql::MysqlConnection,
    r2d2::{ConnectionManager, Pool},
};

#[derive(Clone)]
pub struct Database {
    pool: Pool<ConnectionManager<MysqlConnection>>,
}

impl TypeMapKey for Database {
    type Value = Database;
}

impl AsRef<Database> for Database {
    fn as_ref(&self) -> &Database {
        self
    }
}

impl Database {
    pub async fn new() -> Self {
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        let manager = ConnectionManager::<MysqlConnection>::new(&database_url);
        let pool = Pool::builder()
            .build(manager)
            .expect("Failed to create connection pool");
        Self { pool }
    }
}
