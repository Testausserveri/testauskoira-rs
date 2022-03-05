use diesel::prelude::*;
use serenity::model::channel::Message;

use super::Database;
use crate::models::*;

impl Database {
    pub async fn new_reported_message(
        &self,
        voting_message_id: u64,
        message: Message,
        reporterid: u64,
        mods_online: i32,
    ) -> Result<usize, anyhow::Error> {
        let new_voting = NewCouncilVoting {
            vote_message_id: voting_message_id,
            suspect_id: message.author.id.0,
            suspect_message_id: message.id.0,
            suspect_message_channel_id: message.channel_id.0,
            suspect_message_send_time: message.timestamp.naive_local(),
            suspect_message_content: message.content,
            reporter_id: reporterid,
            delete_votes: 0,
            delete_votes_required: (mods_online as f32).sqrt().clamp(1., 3.).round() as i32,
            silence_votes: 0,
            silence_votes_required: (mods_online as f32).sqrt().round() as i32,
            block_reporter_votes: 0,
            block_reporter_votes_required: (mods_online as f32).sqrt().clamp(1., 3.).round() as i32,
            moderators_online: mods_online,
            useless_clicks: 0,
        };
        Ok(diesel::insert_into(crate::schema::CouncilVotings::table)
            .values(&new_voting)
            .execute(&self.pool.get()?)?)
    }

    pub async fn get_voting_event(
        &self,
        voting_message_id: u64,
    ) -> Result<CouncilVoting, anyhow::Error> {
        use crate::schema::CouncilVotings::dsl::*;
        Ok(CouncilVotings
            .filter(vote_message_id.eq(voting_message_id))
            .first::<CouncilVoting>(&self.pool.get()?)?)
    }

    pub async fn get_voting_event_votes(
        &self,
        voting_messageid: u64,
    ) -> Result<Vec<VotingAction>, anyhow::Error> {
        use crate::schema::VotingActions::dsl::*;
        Ok(VotingActions
            .filter(voting_message_id.eq(voting_messageid))
            .load::<VotingAction>(&self.pool.get()?)?)
    }

    pub async fn get_voting_event_edits(
        &self,
        voting_messageid: u64,
    ) -> Result<Vec<SuspectMessageEdit>, anyhow::Error> {
        use crate::schema::SuspectMessageEdits::dsl::*;
        Ok(SuspectMessageEdits
            .filter(voting_message_id.eq(voting_messageid))
            .order_by(edit_time)
            .load::<SuspectMessageEdit>(&self.pool.get()?)?)
    }

    pub async fn is_reported(&self, message_id: u64) -> Result<bool, anyhow::Error> {
        use crate::schema::CouncilVotings::dsl::*;
        // FIXME: Very Q&D
        Ok(!CouncilVotings
            .filter(suspect_message_id.eq(message_id))
            .load::<CouncilVoting>(&self.pool.get()?)?
            .is_empty())
    }

    pub async fn get_voting_event_for_message(
        &self,
        message_id: u64,
    ) -> Result<CouncilVoting, anyhow::Error> {
        use crate::schema::CouncilVotings::dsl::*;
        Ok(CouncilVotings
            .filter(suspect_message_id.eq(message_id))
            .first::<CouncilVoting>(&self.pool.get()?)?)
    }

    pub async fn add_edit_event(
        &self,
        update_event: serenity::model::event::MessageUpdateEvent,
        voting_message_id: u64,
    ) -> Result<usize, anyhow::Error> {
        let new_edit = NewSuspectMessageEdit {
            voting_message_id,
            suspect_message_id: update_event.id.0,
            new_content: update_event.content.unwrap_or_default(),
            edit_time: update_event.edited_timestamp.unwrap().naive_local(),
        };
        Ok(
            diesel::insert_into(crate::schema::SuspectMessageEdits::table)
                .values(&new_edit)
                .execute(&self.pool.get()?)?,
        )
    }

    pub async fn message_deleted(
        &self,
        delete_time: chrono::NaiveDateTime,
        voting_id: u64,
    ) -> Result<usize, anyhow::Error> {
        let delete = NewSuspectMessageEdit {
            voting_message_id: voting_id,
            suspect_message_id: 0,
            edit_time: delete_time,
            new_content: String::new(),
        };

        Ok(
            diesel::insert_into(crate::schema::SuspectMessageEdits::table)
                .values(&delete)
                .execute(&self.pool.get()?)?,
        )
    }

    pub async fn add_vote(
        &self,
        voting_message_id: u64,
        voter_user_id: u64,
        vote_type: i32,
    ) -> Result<usize, anyhow::Error> {
        let vote = NewVotingAction {
            vote_type,
            voter_user_id,
            voting_message_id,
        };
        {
            use crate::schema::VotingActions::dsl::*;
            // FIXME: REALLY Q&D
            if !VotingActions
                .filter(
                    vote_type
                        .eq(vote.vote_type)
                        .and(voter_user_id.eq(vote.voter_user_id))
                        .and(voting_message_id.eq(vote.voting_message_id)),
                )
                .load::<VotingAction>(&self.pool.get()?)?
                .is_empty()
            {
                return Ok(0);
            }
        }

        diesel::insert_into(crate::schema::VotingActions::table)
            .values(&vote)
            .execute(&self.pool.get()?)?;

        use crate::schema::CouncilVotings::dsl::*;
        match vote_type {
            0 => Ok(
                diesel::update(CouncilVotings.filter(vote_message_id.eq(voting_message_id)))
                    .set(delete_votes.eq(delete_votes + 1))
                    .execute(&self.pool.get()?)?,
            ),
            1 => Ok(
                diesel::update(CouncilVotings.filter(vote_message_id.eq(voting_message_id)))
                    .set(silence_votes.eq(silence_votes + 1))
                    .execute(&self.pool.get()?)?,
            ),
            2 => Ok(
                diesel::update(CouncilVotings.filter(vote_message_id.eq(voting_message_id)))
                    .set(block_reporter_votes.eq(block_reporter_votes + 1))
                    .execute(&self.pool.get()?)?,
            ),
            _ => Ok(0),
        }
    }

    pub async fn remove_vote(
        &self,
        voting_message_id: u64,
        voter_user_id: u64,
        vote_type: i32,
    ) -> Result<usize, anyhow::Error> {
        let vote = NewVotingAction {
            vote_type,
            voter_user_id,
            voting_message_id,
        };
        {
            use crate::schema::VotingActions::dsl::*;
            // FIXME: REALLY Q&D
            if VotingActions
                .filter(
                    vote_type
                        .eq(vote.vote_type)
                        .and(voter_user_id.eq(vote.voter_user_id))
                        .and(voting_message_id.eq(vote.voting_message_id)),
                )
                .load::<VotingAction>(&self.pool.get()?)?
                .is_empty()
            {
                return Ok(0);
            }
            diesel::delete(crate::schema::VotingActions::table)
                .filter(
                    vote_type
                        .eq(vote.vote_type)
                        .and(voter_user_id.eq(vote.voter_user_id))
                        .and(voting_message_id.eq(vote.voting_message_id)),
                )
                .execute(&self.pool.get()?)?;
        }

        use crate::schema::CouncilVotings::dsl::*;
        match vote_type {
            0 => Ok(
                diesel::update(CouncilVotings.filter(vote_message_id.eq(voting_message_id)))
                    .set(delete_votes.eq(delete_votes - 1))
                    .execute(&self.pool.get()?)?,
            ),
            1 => Ok(
                diesel::update(CouncilVotings.filter(vote_message_id.eq(voting_message_id)))
                    .set(silence_votes.eq(silence_votes - 1))
                    .execute(&self.pool.get()?)?,
            ),
            2 => Ok(
                diesel::update(CouncilVotings.filter(vote_message_id.eq(voting_message_id)))
                    .set(block_reporter_votes.eq(block_reporter_votes - 1))
                    .execute(&self.pool.get()?)?,
            ),
            _ => Ok(0),
        }
    }

    pub async fn add_useless_click(&self, message_id: u64) -> Result<usize, anyhow::Error> {
        use crate::schema::CouncilVotings::dsl::*;
        Ok(
            diesel::update(CouncilVotings.filter(vote_message_id.eq(message_id)))
                .set(useless_clicks.eq(useless_clicks + 1))
                .execute(&self.pool.get()?)?,
        )
    }

    pub async fn is_silenced(&self, userid: u64) -> Result<bool, anyhow::Error> {
        use crate::schema::SilencedMembers::dsl::*;
        Ok(SilencedMembers
            .filter(user_id.eq(userid))
            .select(id)
            .first::<i32>(&self.pool.get()?)
            .optional()?
            .is_some())
    }

    pub async fn silence_user(&self, userid: u64) -> Result<usize, anyhow::Error> {
        let new_silence = NewSilencedMember { user_id: userid };
        Ok(diesel::insert_into(crate::schema::SilencedMembers::table)
            .values(&new_silence)
            .execute(&self.pool.get()?)?)
    }

    pub async fn unsilence_user(&self, userid: u64) -> Result<usize, anyhow::Error> {
        use crate::schema::SilencedMembers::dsl::*;
        Ok(
            diesel::delete(SilencedMembers.filter(user_id.eq(userid)))
                .execute(&self.pool.get()?)?,
        )
    }
}
