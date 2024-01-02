use diesel::prelude::*;

use super::Database;
use crate::models::*;

impl Database {
    pub fn new_vote_event(
        &self,
        message_id: u64,
        channel_id: u64,
        author_id: u64,
        title: &str,
        duration: u32,
        options: Vec<String>,
    ) -> Result<i32, anyhow::Error> {
        let event = NewVoteEvent {
            channel_id,
            message_id,
            author_id,
            title: title.to_string(),
            start_time: chrono::Local::now().naive_local(),
            duration,
        };
        diesel::insert_into(crate::schema::VoteEvents::table)
            .values(&event)
            .execute(&self.pool.get()?)?;

        let vote_id = self.get_vote_id_from_message_id(message_id).unwrap();

        for (number, value) in options.iter().enumerate() {
            let option = NewVoteEventOption {
                vote_id,
                option_number: number as i32,
                option_value: value.to_string(),
            };
            diesel::insert_into(crate::schema::VoteEventOptions::table)
                .values(&option)
                .execute(&self.pool.get()?)?;
        }
        Ok(vote_id)
    }

    pub fn get_vote_event_from_message_id(
        &self,
        vote_message_id: u64,
    ) -> Result<VoteEvent, anyhow::Error> {
        use crate::schema::VoteEvents::dsl::*;
        Ok(VoteEvents
            .filter(message_id.eq(vote_message_id))
            .first::<VoteEvent>(&self.pool.get()?)?)
    }

    pub fn get_vote_event_from_id(&self, vote_id: i32) -> Result<VoteEvent, anyhow::Error> {
        use crate::schema::VoteEvents::dsl::*;
        Ok(VoteEvents
            .filter(id.eq(vote_id))
            .first::<VoteEvent>(&self.pool.get()?)?)
    }

    pub fn get_vote_id_from_message_id(&self, vote_message_id: u64) -> Result<i32, anyhow::Error> {
        use crate::schema::VoteEvents::dsl::*;
        Ok(VoteEvents
            .filter(message_id.eq(vote_message_id))
            .select(id)
            .first::<i32>(&self.pool.get()?)?)
    }

    fn get_option_number_by_value(
        &self,
        voteid: i32,
        vote_option_value: &str,
    ) -> Result<i32, anyhow::Error> {
        use crate::schema::VoteEventOptions::dsl::*;
        Ok(VoteEventOptions
            .filter(option_value.eq(vote_option_value).and(vote_id.eq(voteid)))
            .select(option_number)
            .first::<i32>(&self.pool.get()?)?)
    }

    pub fn get_options_by_vote_id(
        &self,
        voteid: i32,
    ) -> Result<Vec<VoteEventOption>, anyhow::Error> {
        use crate::schema::VoteEventOptions::dsl::*;
        Ok(VoteEventOptions
            .filter(vote_id.eq(voteid))
            .load::<VoteEventOption>(&self.pool.get()?)?)
    }

    pub fn get_votes_by_vote_id(&self, voteid: i32) -> Result<Vec<Vote>, anyhow::Error> {
        use crate::schema::Votes::dsl::*;
        Ok(Votes
            .filter(vote_id.eq(voteid))
            .load::<Vote>(&self.pool.get()?)?)
    }

    pub fn user_vote(
        &self,
        vote_message_id: u64,
        voterid: u64,
        option_number: i32,
    ) -> Result<(), anyhow::Error> {
        let db_vote_id = self.get_vote_id_from_message_id(vote_message_id).unwrap();
        // DELETE PREVIOUS VOTE
        {
            use crate::schema::Votes::dsl::*;
            diesel::delete(crate::schema::Votes::table)
                .filter(vote_id.eq(db_vote_id).and(voter_id.eq(voterid)))
                .execute(&self.pool.get()?)?;
        }
        let vote = NewVote {
            vote_id: db_vote_id,
            voter_id: voterid,
            option_number,
        };
        diesel::insert_into(crate::schema::Votes::table)
            .values(&vote)
            .execute(&self.pool.get()?)?;
        Ok(())
    }

    pub fn get_vote_ids(&self) -> Result<Vec<i32>, anyhow::Error> {
        use crate::schema::VoteEvents::dsl::*;
        Ok(VoteEvents.select(id).load::<i32>(&self.pool.get()?)?)
    }

    pub fn purge_vote(&self, voteid: i32) -> Result<(), anyhow::Error> {
        {
            use crate::schema::VoteEventOptions::dsl::*;
            diesel::delete(crate::schema::VoteEventOptions::table)
                .filter(vote_id.eq(voteid))
                .execute(&self.pool.get()?)?;
        }
        {
            use crate::schema::Votes::dsl::*;
            diesel::delete(crate::schema::Votes::table)
                .filter(vote_id.eq(voteid))
                .execute(&self.pool.get()?)?;
        }
        {
            use crate::schema::VoteEvents::dsl::*;
            diesel::delete(crate::schema::VoteEvents::table)
                .filter(id.eq(voteid))
                .execute(&self.pool.get()?)?;
        }
        Ok(())
    }
}
