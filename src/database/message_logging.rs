use super::Database;
use num_traits::cast::ToPrimitive;
use sqlx::mysql::MySqlQueryResult;

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
    pub async fn get_total_messages(&self) -> Result<Option<u64>, sqlx::Error> {
        let mut conn = self.pool.acquire().await?;
        let value = sqlx::query_scalar!(
            "SELECT SUM(`message_count`) FROM `messages_day_stat` WHERE `date` = CURDATE()"
        )
        .fetch_one(&mut conn)
        .await?;

        let value = match value {
            Some(e) => e,
            None => return Ok(None),
        };
        Ok(Some(value.to_u64().unwrap()))
    }
    pub async fn get_most_active(&self, winner_count: u64) -> Result<Vec<(u64, i32)>, sqlx::Error> {
        let mut conn = self.pool.acquire().await?;
        let members = sqlx::query!("SELECT `userid` as `userid!`,`message_count` as `message_count!` FROM `messages_day_stat` WHERE `date` = SUBDATE(CURRENT_DATE, 0) ORDER BY `message_count` DESC LIMIT ?",winner_count)
            .fetch_all(&mut conn)
            .await?;
        let members = members
            .iter()
            .map(|member| (member.userid.parse::<u64>().unwrap(), member.message_count))
            .collect();
        Ok(members)
    }
}
