-- Add migration script here
create type user_role as enum ('admin','member');

CREATE TABLE IF NOT EXISTS users
(
    id               SERIAL PRIMARY KEY         NOT NULL,
    username         VARCHAR(20) UNIQUE         NOT NULL,
    role             user_role default 'member' NOT NULL,

    -- Password hash, generated from:
    -- Argon2::default()
    --   .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
    --   .to_string()
    password_hash    TEXT                       NOT NULL,

    -- A 12 byte long salt used when generating the master key from the password:
    -- let mut key_bytes = [0u8; 32]; // 256-bit key
    -- Argon2::default()
    --     .hash_password_into(password, master_key_salt, &mut key_bytes)
    master_key_salt  bytea                      not null,

    -- A random content key, encrypted using the user's master key:
    -- The salt is 12 bytes long and the encryption uses xchacha20poly1305
    -- content_key_encr = ChaCha20Poly1305::new(master_key)
    --     .encrypt(content_key_salt, content_key)
    content_key_salt bytea                      not null,
    content_key_encr bytea                      not null
);
