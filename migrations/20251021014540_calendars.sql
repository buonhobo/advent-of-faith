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
    id          SERIAL PRIMARY KEY                              NOT NULL,
    calendar_id INT REFERENCES calendars (id) ON DELETE CASCADE NOT NULL,
    unlocks_at  timestamptz                                     NOT NULL,
    protected   bool not null
);

CREATE TABLE IF NOT EXISTS day_content
(
    -- Salt used to derive the decryption key. If this is null then content is unencrypted
    decryption_key_salt bytea,
    -- Encrypted decryption key, derived from owner's day key. If this is null then content is unencrypted
    decryption_key_encr bytea,
    -- Salt used to encrypt the content using the decryption key, if this is null then content is unencrypted
    content_salt bytea,
    -- encrypted or unencrypted content depending on the other values
    content bytea not null,
    day_id int primary key references calendar_days (id) on delete cascade not null
);

CREATE TABLE IF NOT EXISTS calendar_subscriptions
(
    user_id       INT REFERENCES users (id) ON DELETE CASCADE     NOT NULL,
    calendar_id   INT REFERENCES calendars (id) ON DELETE CASCADE NOT NULL,
    subscribed_at timestamptz default now(),
    primary key (user_id, calendar_id)
);

create table if not exists user_days
(
    user_id     int references users (id) on delete cascade         not null,
    day_id      int references calendar_days (id) on delete cascade not null,
    unlocked_at timestamptz default now(),
    -- day key encrypted using xchacha20poly1305 and content key
    day_key_salt bytea,
    day_key_encr bytea,
    primary key (user_id, day_id)
)