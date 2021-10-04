use sqlx::MySqlPool;
pub mod message_logging;


pub struct Database {
    pool: MySqlPool,
}

impl Database {
    pub async fn new() -> Database {
        let pool = MySqlPool::connect(&std::env::var("DATABASE_URL").unwrap()).await.unwrap();
        Self { pool }
    }
}
