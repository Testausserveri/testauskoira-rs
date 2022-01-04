#[derive(Queryable, Clone, Debug)]
pub struct UserMessageStat {
    pub id: i32,
    pub date: chrono::NaiveDate,
    pub userid: String,
    pub message_count: i32,
}

use crate::schema::messages_day_stat;

#[derive(Insertable)]
#[table_name = "messages_day_stat"]
pub struct NewUserMessageStat {
    pub date: chrono::NaiveDate,
    pub userid: String,
    pub message_count: i32,
}
