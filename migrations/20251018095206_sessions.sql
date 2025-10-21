-- Add migration script here
CREATE TABLE IF NOT EXISTS user_sessions
(
    token_hash CHAR(64) PRIMARY KEY,
    user_id    INT REFERENCES users (id) ON DELETE CASCADE    NOT NULL,
    created_at timestamptz DEFAULT now()                      NOT NULL,
    expires_at timestamptz DEFAULT (now() + interval '1 day') NOT NULL
);