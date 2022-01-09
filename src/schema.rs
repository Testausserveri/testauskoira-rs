table! {
    CouncilVotings (id) {
        id -> Integer,
        vote_message_id -> Unsigned<Bigint>,
        suspect_id -> Unsigned<Bigint>,
        suspect_message_id -> Unsigned<Bigint>,
        suspect_message_channel_id -> Unsigned<Bigint>,
        suspect_message_send_time -> Datetime,
        suspect_message_content -> Text,
        reporter_id -> Unsigned<Bigint>,
        delete_votes -> Integer,
        delete_votes_required -> Integer,
        silence_votes -> Integer,
        silence_votes_required -> Integer,
        block_reporter_votes -> Integer,
        block_reporter_votes_required -> Integer,
        moderators_online -> Integer,
    }
}

table! {
    SuspectMessageEdits (id) {
        id -> Integer,
        voting_message_id -> Unsigned<Bigint>,
        suspect_message_id -> Unsigned<Bigint>,
        new_content -> Text,
        edit_time -> Datetime,
    }
}

table! {
    VotingActions (id) {
        id -> Integer,
        vote_type -> Integer,
        voter_user_id -> Unsigned<Bigint>,
        voting_message_id -> Unsigned<Bigint>,
    }
}

table! {
    messages_day_stat (id) {
        id -> Integer,
        date -> Nullable<Date>,
        userid -> Nullable<Varchar>,
        message_count -> Nullable<Integer>,
    }
}

allow_tables_to_appear_in_same_query!(
    CouncilVotings,
    SuspectMessageEdits,
    VotingActions,
    messages_day_stat,
);
