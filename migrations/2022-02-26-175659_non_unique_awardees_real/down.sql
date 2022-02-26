-- If multiple non-unique ids exist this revert
-- will fail (for obvious reasons) rendering it almost useless

CREATE TABLE TempAwardWinners(
    id INTEGER UNIQUE NOT NULL AUTO_INCREMENT,
    user_id BIGINT UNSIGNED UNIQUE NOT NULL,
    date DATE NOT NULL,
    PRIMARY KEY (id)
);

INSERT INTO TempAwardWinners (user_id, date)
SELECT user_id, date FROM AwardWinners;
DROP TABLE AwardWinners;
ALTER TABLE TempAwardWinners RENAME TO AwardWinners;
