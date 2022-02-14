use diesel::prelude::*;

use super::Database;
use crate::models::*;

impl Database {
    pub async fn increment_message_count(&self, in_userid: &u64) -> Result<usize, anyhow::Error> {
        // FIXME: This could be optimized if necessary
        let curdate = chrono::Local::today().naive_local();
        use crate::schema::messages_day_stat::dsl::*;
        let current_count = messages_day_stat
            .filter(userid.eq(in_userid.to_string()).and(date.eq(curdate)))
            .select(message_count)
            .first::<Option<i32>>(&self.pool.get()?)
            .unwrap_or(None);

        Ok(match current_count {
            Some(c) => diesel::update(
                messages_day_stat.filter(userid.eq(in_userid.to_string()).and(date.eq(curdate))),
            )
            .set(message_count.eq(c + 1))
            .execute(&self.pool.get()?)?,
            None => {
                let new_entry = crate::models::NewUserMessageStat {
                    date: curdate,
                    userid: in_userid.to_string(),
                    message_count: 1,
                };
                use crate::schema::messages_day_stat;
                diesel::insert_into(messages_day_stat::table)
                    .values(&new_entry)
                    .execute(&self.pool.get()?)?
            }
        })
    }
    pub async fn get_total_daily_messages(&self, offset: i32) -> Result<i64, anyhow::Error> {
        let curdate = chrono::Local::today().naive_local() - chrono::Duration::days(offset.into());
        use crate::schema::messages_day_stat::dsl::*;

        let value = messages_day_stat
            .filter(date.eq(curdate))
            .select(diesel::dsl::sum(message_count))
            .first::<Option<i64>>(&self.pool.get()?)?;

        let value = value.unwrap_or(0);
        Ok(value)
    }
    pub async fn get_most_active(
        &self,
        winner_count: i64,
        days_pre: i32,
    ) -> Result<Vec<(u64, i32)>, anyhow::Error> {
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
        let blacklist = blacklist.lines();
        let curdate =
            chrono::Local::today().naive_local() - chrono::Duration::days(days_pre.into());

        use crate::schema::messages_day_stat::dsl::*;

        let members = messages_day_stat
            .filter(date.eq(curdate).and(userid.ne_all(blacklist)))
            .select((userid, message_count))
            .order(message_count.desc())
            .limit(winner_count)
            .load::<(Option<String>, Option<i32>)>(&self.pool.get()?)?;

        let members = members
            .iter()
            .map(|m| (m.0.as_ref().unwrap().parse::<u64>().unwrap(), m.1.unwrap()))
            .collect();
        Ok(members)
    }

    pub async fn get_total_message_average(&self, offset: i32) -> Result<f32, anyhow::Error> {
        let curdate = chrono::Local::today().naive_local() - chrono::Duration::days(offset.into());
        use crate::schema::messages_day_stat::dsl::*;

        let mut res = messages_day_stat
            .filter(date.lt(curdate))
            .group_by(date)
            .select(diesel::dsl::sum(message_count))
            .load::<Option<i64>>(&self.pool.get()?)?;

        res.retain(|x| x.is_some());
        // Haskell :hear_eyes:
        Ok(res.iter().fold(0, |acc, x| acc + x.unwrap()) as f32 / res.len() as f32)
    }

    pub async fn get_last_winner(&self) -> Result<u64, anyhow::Error> {
        use crate::schema::AwardWinners::dsl::*;
        Ok(AwardWinners
            .order_by(date)
            .select(user_id)
            .first(&self.pool.get()?)?)
    }

    pub async fn new_winner(&self, id: u64) -> Result<usize, anyhow::Error> {
        let curdate = chrono::Local::today().naive_local();
        let new_winner = NewAwardWinner {
            user_id: id,
            date: curdate,
        };
        Ok(diesel::insert_into(crate::schema::AwardWinners::table)
            .values(&new_winner)
            .execute(&self.pool.get()?)?)
    }
}
