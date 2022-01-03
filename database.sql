CREATE DATABASE Testauskoira;
USE Testauskoira;

CREATE TABLE messages_day_stat(
  id INT(11) NOT NULL AUTO_INCREMENT,
  date DATE DEFAULT NULL,
  userid VARCHAR(20) DEFAULT NULL,
  message_count INT(11) DEFAULT NULL,
  PRIMARY KEY (id),
  UNIQUE KEY userid (userid, date) USING HASH
);
