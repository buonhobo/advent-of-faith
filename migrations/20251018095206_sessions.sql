-- Add migration script here
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS user_sessions
(
    id         UUID        DEFAULT uuid_generate_v4() PRIMARY KEY,
    user_id    INT REFERENCES users (id) ON DELETE CASCADE         NOT NULL,
    created_at timestamptz DEFAULT now()                           NOT NULL,
    expires_at timestamptz DEFAULT (now() + interval '1 day') NOT NULL
);