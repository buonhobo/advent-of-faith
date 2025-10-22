-- Add migration script here
CREATE TABLE IF NOT EXISTS calendars
(
    id         SERIAL PRIMARY KEY                          NOT NULL,
    owner_id   INT REFERENCES users (id) ON DELETE CASCADE NOT NULL,
    created_at timestamptz DEFAULT now()                   NOT NULL,
    title      text                                        NOT NULL
);

CREATE TABLE IF NOT EXISTS calendar_days
(
    id           SERIAL PRIMARY KEY                              NOT NULL,
    calendar_id  INT REFERENCES calendars (id) ON DELETE CASCADE NOT NULL,
    unlocks_at   timestamptz                                     NOT NULL,
    -- if day_key_hash is null, then content is unencrypted json
    content      bytea                                           not null,
    -- argon2 hash of day key
    day_key_hash text,
    -- salt used to encrypt the content using xchacha20poly1305 and day key
    content_salt bytea
);

CREATE TABLE IF NOT EXISTS calendar_subscriptions
(
    user_id       INT REFERENCES users (id) ON DELETE CASCADE     NOT NULL,
    calendar_id   INT REFERENCES calendars (id) ON DELETE CASCADE NOT NULL,
    subscribed_at timestamptz                                     NOT NULL,
    primary key (user_id, calendar_id)
);

create table if not exists user_days
(
    user_id      int references users (id) on delete cascade         not null,
    day_id       int references calendar_days (id) on delete cascade not null,
    unlocked_at  timestamptz,
    -- day key encrypted using xchacha20poly1305 and content key
    day_key_salt bytea,
    day_key_encr bytea,
    primary key (user_id, day_id)
)