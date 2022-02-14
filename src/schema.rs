#![allow(non_snake_case)]
table! {
    AwardWinners (id) {
        id -> Integer,
        user_id -> Unsigned<Bigint>,
        date -> Date,
    }
}

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
        useless_clicks -> Bigint,
    }
}

table! {
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

table! {
    GiveawayWinners (id) {
        id -> Bigint,
        giveaway_id -> Bigint,
        user_id -> Unsigned<Bigint>,
        rerolled -> Bool,
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

joinable!(GiveawayWinners -> Giveaways (giveaway_id));

allow_tables_to_appear_in_same_query!(
    AwardWinners,
    CouncilVotings,
    Giveaways,
    GiveawayWinners,
    messages_day_stat,
    SuspectMessageEdits,
    VotingActions,
);
