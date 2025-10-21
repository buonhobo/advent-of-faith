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
    unlock_at   timestamptz                                     NOT NULL,
    code_hash   CHAR(64),
    title       text,
    content     text
);

CREATE TABLE IF NOT EXISTS calendar_subscriptions
(
    user_id       INT REFERENCES users (id) ON DELETE CASCADE     NOT NULL,
    calendar_id   INT REFERENCES calendars (id) ON DELETE CASCADE NOT NULL,
    subscribed_at timestamptz                                     NOT NULL,
    primary key (user_id, calendar_id)
);