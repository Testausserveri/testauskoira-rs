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

#[derive(Queryable, Clone, Debug)]
pub struct CouncilVoting {
    pub id: i32,
    pub vote_message_id: u64,
    pub suspect_id: u64,
    pub suspect_message_id: u64,
    pub suspect_message_channel_id: u64,
    pub suspect_message_send_time: chrono::NaiveDateTime,
    pub suspect_message_content: String,
    pub reporter_id: u64,
    pub delete_votes: i32,
    pub delete_votes_required: i32,
    pub silence_votes: i32,
    pub silence_votes_required: i32,
    pub block_reporter_votes: i32,
    pub block_reporter_votes_required: i32,
    pub moderators_online: i32,
}

use crate::schema::CouncilVotings;

#[derive(Insertable)]
#[table_name = "CouncilVotings"]
pub struct NewCouncilVoting {
    pub vote_message_id: u64,
    pub suspect_id: u64,
    pub suspect_message_id: u64,
    pub suspect_message_channel_id: u64,
    pub suspect_message_send_time: chrono::NaiveDateTime,
    pub suspect_message_content: String,
    pub reporter_id: u64,
    pub delete_votes: i32,
    pub delete_votes_required: i32,
    pub silence_votes: i32,
    pub silence_votes_required: i32,
    pub block_reporter_votes: i32,
    pub block_reporter_votes_required: i32,
    pub moderators_online: i32,
}

#[derive(Queryable, Clone, Debug)]
pub struct VotingAction {
    pub id: i32,
    pub vote_type: i32,
    pub voter_user_id: u64,
    pub voting_message_id: u64,
}

use crate::schema::VotingActions;

#[derive(Insertable)]
#[table_name = "VotingActions"]
pub struct NewVotingAction {
    pub vote_type: i32,
    pub voter_user_id: u64,
    pub voting_message_id: u64,
}

#[derive(Queryable, Clone, Debug)]
pub struct SuspectMessageEdit {
    pub id: i32,
    pub voting_message_id: u64,
    pub suspect_message_id: u64,
    pub new_content: String,
    pub edit_time: chrono::NaiveDateTime,
}

use crate::schema::SuspectMessageEdits;

#[derive(Insertable)]
#[table_name = "SuspectMessageEdits"]
pub struct NewSuspectMessageEdit {
    pub voting_message_id: u64,
    pub suspect_message_id: u64,
    pub new_content: String,
    pub edit_time: chrono::NaiveDateTime,
}
