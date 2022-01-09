ALTER TABLE CouncilVotings
MODIFY vote_message_id BIGINT UNIQUE NOT NULL,
MODIFY suspect_id BIGINT NOT NULL,
MODIFY suspect_message_id BIGINT NOT NULL,
MODIFY suspect_message_channel_id BIGINT NOT NULL,
MODIFY reporter_id BIGINT NOT NULL;

ALTER TABLE VotingActions
MODIFY voter_user_id BIGINT NOT NULL,
MODIFY voting_message_id BIGINT NOT NULL;

ALTER TABLE SuspectMessageEdits
MODIFY voting_message_id BIGINT NOT NULL,
MODIFY suspect_message_id BIGINT NOT NULL;
