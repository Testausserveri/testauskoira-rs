use num_traits::cast::ToPrimitive;
use sqlx::mysql::MySqlQueryResult;

use super::Database;

#[derive(sqlx::FromRow)]
struct Member {
    message_count: i32,
    userid: String,
}

impl Database {
    pub async fn increment_message_count(
        &self,
        userid: &u64,
    ) -> Result<MySqlQueryResult, sqlx::Error> {
        let mut conn = self.pool.acquire().await.unwrap();
        sqlx::query!("INSERT INTO `messages_day_stat` SET `message_count` = 1, `userid` = ?, `date` = CURDATE() ON DUPLICATE KEY UPDATE `message_count` = `message_count` + 1",
        userid.to_string())
            .execute(&mut conn)
            .await
    }
    pub async fn get_total_daily_messages(&self) -> Result<u64, sqlx::Error> {
        let mut conn = self.pool.acquire().await?;
        let value = sqlx::query_scalar!(
            "SELECT SUM(`message_count`) FROM `messages_day_stat` WHERE `date` = CURDATE()"
        )
        .fetch_one(&mut conn)
        .await?;

        let value = match value {
            Some(e) => e.to_u64().unwrap(),
            None => 0,
        };
        Ok(value)
    }
    pub async fn get_most_active(&self, winner_count: u64, days_pre: i32) -> Result<Vec<(u64, i32)>, sqlx::Error> {
        let mut conn = self.pool.acquire().await?;
        let blacklist = match std::fs::read_to_string("award_id_blacklist.txt") {
            Ok(s) => s,
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        std::fs::File::create("award_id_blacklist.txt")
                            .expect("Unable to create award_id_blacklist.txt");
                    }
                    _ => panic!("Unable to access award_id_blacklist.txt"),
                }
                String::new()
            }
        };
        let blacklist = blacklist
            .lines()
            .map(|s| format!("'{}'", s))
            .collect::<Vec<_>>()
            .join(",");

        let members: Vec<Member> = sqlx::query_as(&format!("SELECT `userid`,`message_count` FROM `messages_day_stat` WHERE `date` = SUBDATE(CURRENT_DATE, {}) AND `userid` NOT IN ( {} ) ORDER BY `message_count` DESC LIMIT {}", days_pre, &blacklist, winner_count))
            .fetch_all(&mut conn)
            .await?;
        let members = members
            .iter()
            .map(|m| (m.userid.parse::<u64>().unwrap(), m.message_count))
            .collect();
        Ok(members)
    }
}
