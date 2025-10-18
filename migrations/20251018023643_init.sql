-- Add migration script here
create type user_role as enum ('admin','member');

CREATE TABLE IF NOT EXISTS users
(
    id            SERIAL PRIMARY KEY         NOT NULL,
    username      VARCHAR(20) UNIQUE         NOT NULL,
    password_hash TEXT                       NOT NULL,
    role          user_role default 'member' NOT NULL
);