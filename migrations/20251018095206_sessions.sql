-- Add migration script here
CREATE TABLE IF NOT EXISTS user_sessions
(
    -- Sha256 hash of session token
    token_hash      CHAR(64) PRIMARY KEY,
    user_id         INT REFERENCES users (id) ON DELETE CASCADE    NOT NULL,
    created_at      timestamptz DEFAULT now()                      NOT NULL,
    expires_at      timestamptz DEFAULT (now() + interval '1 day') NOT NULL,

    -- encrypted master key using session token and xchacha20poly1305
    master_key_encr bytea                                          not null,
    master_key_salt bytea                                          not null
);