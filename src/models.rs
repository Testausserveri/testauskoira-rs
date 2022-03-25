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
    pub useless_clicks: i64,
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
    pub useless_clicks: i64,
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

use crate::schema::Giveaways;

#[derive(Queryable)]
pub struct Giveaway {
    pub id: i64,
    pub message_id: u64,
    pub channel_id: u64,
    pub start_time: chrono::NaiveDateTime,
    pub end_time: chrono::NaiveDateTime,
    pub max_winners: i64,
    pub prize: String,
    pub completed: bool,
}

#[derive(Insertable)]
#[table_name = "Giveaways"]
pub struct NewGiveaway {
    pub message_id: u64,
    pub channel_id: u64,
    pub end_time: chrono::NaiveDateTime,
    pub max_winners: i64,
    pub prize: String,
}

use crate::schema::GiveawayWinners;

#[derive(Queryable)]
pub struct GiveawayWinner {
    pub id: i64,
    pub giveaway_id: i64,
    pub user_id: u64,
    pub rerolled: bool,
}

#[derive(Insertable)]
#[table_name = "GiveawayWinners"]
pub struct NewGiveawayWinner {
    pub giveaway_id: i64,
    pub user_id: u64,
}

#[derive(Queryable)]
pub struct AwardWinner {
    pub id: i32,
    pub user_id: u64,
    pub date: chrono::NaiveDate,
}

use crate::schema::AwardWinners;

#[derive(Insertable)]
#[table_name = "AwardWinners"]
pub struct NewAwardWinner {
    pub user_id: u64,
    pub date: chrono::NaiveDate,
}

#[derive(Queryable)]
pub struct SilencedMember {
    pub id: i32,
    pub user_id: u64,
}

use crate::schema::SilencedMembers;

#[derive(Insertable)]
#[table_name = "SilencedMembers"]
pub struct NewSilencedMember {
    pub user_id: u64,
}

#[derive(Queryable)]
pub struct VoteEvent {
    pub id: i32,
    pub title: String,
    pub message_id: u64,
    pub channel_id: u64,
    pub author_id: u64,
    pub start_time: chrono::NaiveDateTime,
    pub duration: u32,
}

use crate::schema::VoteEvents;

#[derive(Insertable)]
#[table_name = "VoteEvents"]
pub struct NewVoteEvent {
    pub message_id: u64,
    pub channel_id: u64,
    pub author_id: u64,
    pub title: String,
    pub start_time: chrono::NaiveDateTime,
    pub duration: u32,
}

#[derive(Queryable)]
pub struct VoteEventOption {
    pub id: i32,
    pub vote_id: i32,
    pub option_number: i32,
    pub option_value: String,
}

use crate::schema::VoteEventOptions;

#[derive(Insertable)]
#[table_name = "VoteEventOptions"]
pub struct NewVoteEventOption {
    pub vote_id: i32,
    pub option_number: i32,
    pub option_value: String,
}

#[derive(Queryable)]
pub struct Vote {
    pub id: i32,
    pub vote_id: i32,
    pub voter_id: u64,
    pub option_number: i32,
}

use crate::schema::Votes;

#[derive(Insertable)]
#[table_name = "Votes"]
pub struct NewVote {
    pub vote_id: i32,
    pub voter_id: u64,
    pub option_number: i32,
}
