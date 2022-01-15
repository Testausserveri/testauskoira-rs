use chrono::NaiveDateTime;
use diesel::prelude::*;

use super::Database;
use crate::{models::*, Message};

impl Database {
    pub async fn start_giveaway<S: Into<String>>(
        &self,
        message: &Message,
        end_time: NaiveDateTime,
        max_winners: i64,
        prize: S,
    ) -> Result<i64, anyhow::Error> {
        use crate::schema::{Giveaways as GiveawaysSchema, Giveaways::dsl::Giveaways};
        let giveaway = NewGiveaway {
            message_id: message.id.0,
            channel_id: message.channel_id.0,
            end_time,
            max_winners,
            prize: prize.into(),
        };

        diesel::insert_into(GiveawaysSchema::table)
            .values(&giveaway)
            .execute(&self.pool.get()?)?;

        Ok(Giveaways
            .filter(GiveawaysSchema::message_id.eq(message.id.0))
            .select(GiveawaysSchema::id)
            .first::<i64>(&self.pool.get()?)?)
    }

    pub async fn end_giveaway(&self, giveaway_id: i64) -> Result<(), anyhow::Error> {
        use crate::schema::Giveaways;

        diesel::update(Giveaways::table)
            .filter(Giveaways::id.eq(giveaway_id))
            .set(Giveaways::completed.eq(true))
            .execute(&self.pool.get()?)?;

        Ok(())
    }

    pub async fn delete_giveaway(&self, giveaway_id: i64) -> Result<Giveaway, anyhow::Error> {
        use crate::schema::{Giveaways as GiveawaysSchema, Giveaways::dsl::Giveaways};

        let giveaway = Giveaways
            .filter(GiveawaysSchema::id.eq(giveaway_id))
            .first::<Giveaway>(&self.pool.get()?)?;

        diesel::delete(GiveawaysSchema::table)
            .filter(GiveawaysSchema::id.eq(giveaway_id))
            .execute(&self.pool.get()?)?;

        Ok(giveaway)
    }

    pub async fn get_giveaway(&self, giveaway_id: i64) -> Result<Giveaway, anyhow::Error> {
        use crate::schema::Giveaways::dsl::*;

        Ok(Giveaways
            .filter(id.eq(giveaway_id))
            .first::<Giveaway>(&self.pool.get()?)?)
    }

    pub async fn get_n_giveaways_with_offset(
        &self,
        n: i64,
        offset: i64,
    ) -> Result<Vec<Giveaway>, anyhow::Error> {
        use crate::schema::Giveaways::dsl::*;

        Ok(Giveaways
            .limit(n)
            .offset(offset)
            .load::<Giveaway>(&self.pool.get()?)?)
    }

    pub async fn get_giveaways(&self) -> Result<Vec<Giveaway>, anyhow::Error> {
        use crate::schema::Giveaways::dsl::*;
        Ok(Giveaways.load::<Giveaway>(&self.pool.get()?)?)
    }

    pub async fn get_ongoing_giveaways(&self) -> Result<Vec<Giveaway>, anyhow::Error> {
        use crate::schema::Giveaways::dsl::*;
        Ok(Giveaways
            .filter(completed.eq(false))
            .load::<Giveaway>(&self.pool.get()?)?)
    }

    pub async fn set_giveaway_completed(
        &self,
        giveaway_id: i64,
        completed: bool,
    ) -> Result<(), anyhow::Error> {
        use crate::schema::Giveaways;

        diesel::update(Giveaways::table)
            .filter(Giveaways::id.eq(giveaway_id))
            .set(Giveaways::completed.eq(completed))
            .execute(&self.pool.get()?)?;

        Ok(())
    }

    pub async fn set_giveaway_winners(
        &self,
        giveaway_id: i64,
        winners: &Vec<u64>,
    ) -> Result<(), anyhow::Error> {
        use crate::schema::GiveawayWinners;

        diesel::delete(GiveawayWinners::table)
            .filter(GiveawayWinners::giveaway_id.eq(giveaway_id))
            .execute(&self.pool.get()?)?;

        diesel::insert_into(GiveawayWinners::table)
            .values(
                winners
                    .iter()
                    .map(|&id| NewGiveawayWinner {
                        giveaway_id,
                        user_id: id,
                    })
                    .collect::<Vec<NewGiveawayWinner>>(),
            )
            .execute(&self.pool.get()?)?;

        Ok(())
    }

    pub async fn edit_giveaway_duration(
        &self,
        giveaway_id: i64,
        new_value: NaiveDateTime,
    ) -> Result<Giveaway, anyhow::Error> {
        use crate::schema::Giveaways::dsl::*;

        diesel::update(Giveaways.filter(id.eq(giveaway_id)))
            .set(end_time.eq(new_value))
            .execute(&self.pool.get()?)?;

        Ok(Giveaways
            .filter(id.eq(giveaway_id))
            .first::<Giveaway>(&self.pool.get()?)?)
    }

    pub async fn edit_giveaway_max_winners(
        &self,
        giveaway_id: i64,
        new_value: i64,
    ) -> Result<Giveaway, anyhow::Error> {
        use crate::schema::Giveaways::dsl::*;

        diesel::update(Giveaways.filter(id.eq(giveaway_id)))
            .set(max_winners.eq(new_value))
            .execute(&self.pool.get()?)?;

        Ok(Giveaways
            .filter(id.eq(giveaway_id))
            .first::<Giveaway>(&self.pool.get()?)?)
    }

    pub async fn get_giveaway_winners(
        &self,
        filter_giveaway_id: i64,
    ) -> Result<Vec<GiveawayWinner>, anyhow::Error> {
        use crate::schema::GiveawayWinners::dsl::*;

        Ok(GiveawayWinners
            .filter(giveaway_id.eq(filter_giveaway_id))
            .load::<GiveawayWinner>(&self.pool.get()?)?)
    }
}
