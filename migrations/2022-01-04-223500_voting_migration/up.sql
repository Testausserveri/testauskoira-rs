CREATE TABLE CouncilVotings(
    id INTEGER UNIQUE NOT NULL AUTO_INCREMENT,
    vote_message_id BIGINT UNIQUE NOT NULL,
    suspect_id BIGINT NOT NULL,
    suspect_message_id BIGINT NOT NULL,
    suspect_message_channel_id BIGINT NOT NULL,
    suspect_message_send_time DATETIME NOT NULL,
    suspect_message_content TEXT NOT NULL,
    reporter_id BIGINT NOT NULL,
    delete_votes INTEGER NOT NULL DEFAULT 0,
    delete_votes_required INTEGER NOT NULL,
    silence_votes INTEGER NOT NULL DEFAULT 0,
    silence_votes_required INTEGER NOT NULL,
    block_reporter_votes INTEGER NOT NULL DEFAULT 0,
    block_reporter_votes_required INTEGER NOT NULL,
    moderators_online INTEGER NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE VotingActions(
    id INTEGER UNIQUE NOT NULL AUTO_INCREMENT,
    vote_type INTEGER NOT NULL,
    voter_user_id BIGINT NOT NULL,
    voting_message_id BIGINT NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE SuspectMessageEdits(
    id INTEGER UNIQUE NOT NULL AUTO_INCREMENT,
    voting_message_id BIGINT NOT NULL,
    suspect_message_id BIGINT NOT NULL,
    new_content TEXT NOT NULL,
    edit_time DATETIME NOT NULL,
    PRIMARY KEY (id)
);
