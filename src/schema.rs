#![allow(non_snake_case)]
diesel::table! {
    AwardWinners (id) {
        id -> Integer,
        user_id -> Unsigned<Bigint>,
        date -> Date,
    }
}

diesel::table! {
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
        useless_clicks -> Bigint,
    }
}

diesel::table! {
    Giveaways (id) {
        id -> Bigint,
        message_id -> Unsigned<Bigint>,
        channel_id -> Unsigned<Bigint>,
        start_time -> Datetime,
        end_time -> Datetime,
        max_winners -> Bigint,
        prize -> Text,
        completed -> Bool,
    }
}

diesel::table! {
    GiveawayWinners (id) {
        id -> Bigint,
        giveaway_id -> Bigint,
        user_id -> Unsigned<Bigint>,
        rerolled -> Bool,
    }
}

diesel::table! {
    messages_day_stat (id) {
        id -> Integer,
        date -> Nullable<Date>,
        userid -> Nullable<Varchar>,
        message_count -> Nullable<Integer>,
    }
}

diesel::table! {
    SuspectMessageEdits (id) {
        id -> Integer,
        voting_message_id -> Unsigned<Bigint>,
        suspect_message_id -> Unsigned<Bigint>,
        new_content -> Text,
        edit_time -> Datetime,
    }
}

diesel::table! {
    VoteEventOptions (id) {
        id -> Integer,
        vote_id -> Integer,
        option_number -> Integer,
        option_value -> Varchar,
    }
}

diesel::table! {
    VoteEvents (id) {
        id -> Integer,
        title -> Tinytext,
        message_id -> Unsigned<Bigint>,
        channel_id -> Unsigned<Bigint>,
        author_id -> Unsigned<Bigint>,
        start_time -> Datetime,
        duration -> Unsigned<Integer>,
    }
}

diesel::table! {
    Votes (id) {
        id -> Integer,
        vote_id -> Integer,
        voter_id -> Unsigned<Bigint>,
        option_number -> Integer,
    }
}

diesel::table! {
    VotingActions (id) {
        id -> Integer,
        vote_type -> Integer,
        voter_user_id -> Unsigned<Bigint>,
        voting_message_id -> Unsigned<Bigint>,
    }
}

diesel::joinable!(GiveawayWinners -> Giveaways (giveaway_id));
diesel::joinable!(VoteEventOptions -> VoteEvents (vote_id));
diesel::joinable!(Votes -> VoteEvents (vote_id));

diesel::allow_tables_to_appear_in_same_query!(
    AwardWinners,
    CouncilVotings,
    Giveaways,
    GiveawayWinners,
    messages_day_stat,
    SuspectMessageEdits,
    VoteEventOptions,
    VoteEvents,
    Votes,
    VotingActions,
);
