use sqlx::mysql::MySqlQueryResult;
use rand::seq::IteratorRandom;

use super::Database;

pub struct Mailbox {
    pub name: String,
    pub sub: String,
}

impl Database {
    pub async fn get_registered_users(&self) -> Result<Vec<(String,u64)>, sqlx::Error> {
        let mut conn = self.pool.acquire().await.unwrap();
        let reg_users = sqlx::query!("SELECT `mailbox` as `mailbox!`, `userid` as `userid!` FROM `mailboxes`")
            .fetch_all(&mut conn)
            .await?;
        let reg_users = reg_users.iter().map(|user| {
            (user.mailbox.clone(),user.userid.parse::<u64>().unwrap())
        }).collect();
        Ok(reg_users)
    }
    pub async fn get_user_by_mailbox(&self, mailbox: &str) -> Option<u64> {
        let mut conn = self.pool.acquire().await.unwrap();
        let owner = sqlx::query_scalar!("SELECT `userid` as `userid!` FROM `mailboxes` WHERE `mailbox` = ?", mailbox)
            .fetch_optional(&mut conn)
            .await;
        match owner.unwrap() {
            Some(str) => Some(str.parse::<u64>().unwrap()),
            None => None,
        }
    }
    pub async fn get_mailbox_by_key(&self, key: &str) -> Option<String> { 
        let mut conn = self.pool.acquire().await.unwrap();
        let mailbox = sqlx::query!("SELECT `mailbox` FROM `mailboxes` WHERE `key` = ?",key)
            .fetch_optional(&mut conn)
            .await;
        match mailbox.unwrap() {
            Some(m) => m.mailbox,
            None => None,
        }
    }
    pub async fn available(&self, mailbox: &str) -> bool {
        let mut conn = self.pool.acquire().await.unwrap();
        let mailbox = sqlx::query!("SELECT `mailbox` FROM `mailboxes` WHERE `mailbox` = ?", mailbox)
            .fetch_optional(&mut conn)
            .await;
        let mailbox = mailbox.unwrap();
        match mailbox {
            Some(_) => false,
            None => true,
        }
    }
    pub async fn create_mailbox(&self,mailbox: &str,userid: &u64) -> Result<MySqlQueryResult,sqlx::Error> {
        let hex = "0123456789abcdef";
        let mut rng = rand::thread_rng();
        let key = (0..40).map(|_| {
            hex.chars().choose(&mut rng).unwrap()
        }).collect::<String>();

        let mut conn = self.pool.acquire().await?;
        sqlx::query!("INSERT INTO `mailboxes` SET `mailbox`=?, `userid`=?, `key`=?",mailbox,userid.to_string(),key)
            .execute(&mut conn)
            .await
    }
    pub async fn add_block(&self, from: &str, mailbox: &str, sub: &str) -> Result<MySqlQueryResult, sqlx::Error> {
        let mut conn = self.pool.acquire().await.unwrap();
        sqlx::query!("INSERT INTO `blocks` SET `from`=?, `mailbox`=?, `sub`=?", from, mailbox, sub)
            .execute(&mut conn)
            .await
    }
    pub async fn check_block(&self, from: &str, mailbox: &Mailbox) -> Option<i32> {
        let mut conn = self.pool.acquire().await.unwrap();
        let query = sqlx::query!("SELECT `id` FROM `blocks` WHERE `from`=? AND `mailbox`=? AND `sub`=?",from, mailbox.name, mailbox.sub)
            .fetch_optional(&mut conn)
            .await;
        match query.unwrap() {
            Some(q) => Some(q.id),
            None => None,
        }
    }
}
