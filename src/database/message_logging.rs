use super::Database;
use sqlx::types::BigDecimal;
use num_traits::cast::ToPrimitive;

impl Database {
    pub async fn increment_message_count(&self, id: serenity::model::id::UserId) {
        let mut conn = self.pool.acquire().await.unwrap();
        sqlx::query!("INSERT INTO `messages_day_stat` SET `message_count`=1, `userid`=?, `date` = CURDATE() ON DUPLICATE KEY UPDATE `message_count`=`message_count`+1; ",
            id.as_u64())
            .execute(&mut conn)
            .await
            .unwrap();
    }
    pub async fn get_total_messages(&self) -> u64 {
        let mut conn = self.pool.acquire().await.unwrap();
        let value: BigDecimal = sqlx::query_scalar!("SELECT SUM(`message_count`) FROM `messages_day_stat` WHERE `date` = CURDATE()")
            .fetch_optional(&mut conn)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        value.to_u64().unwrap()
    }
}
