pub mod giveaway;
pub mod message_logging;
pub mod vote;
pub mod voting;

use diesel::{
    mysql::MysqlConnection,
    r2d2::{ConnectionManager, Pool},
    Connection, RunQueryDsl,
};

embed_migrations!();

#[derive(Clone)]
pub struct Database {
    pool: Pool<ConnectionManager<MysqlConnection>>,
}

impl AsRef<Database> for Database {
    fn as_ref(&self) -> &Database {
        self
    }
}

impl Database {
    pub async fn new() -> Self {
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        // Auto-create the database so fresh deployments don't require manual provisioning
        let db_name = database_url.split('?').next().unwrap()
            .rsplit('/')
            .next()
            .expect("DATABASE_URL must end with /database_name");
        let server_url = &database_url[..database_url.len() - db_name.len() - 1];
        {
            let conn = MysqlConnection::establish(server_url)
                .expect("Failed to connect to MySQL server");
            diesel::sql_query(format!("CREATE DATABASE IF NOT EXISTS `{db_name}`"))
                .execute(&conn)
                .expect("Failed to create database");
        }

        let manager = ConnectionManager::<MysqlConnection>::new(&database_url);
        let pool = Pool::builder()
            .build(manager)
            .expect("Failed to create connection pool");

        let conn = pool.get().expect("Failed to get connection for migrations");
        embedded_migrations::run(&conn).expect("Failed to run database migrations");

        Self { pool }
    }
}
