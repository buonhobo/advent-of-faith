use crate::model::user::{User, UserRole};
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chacha20poly1305::aead::AeadMut;
use chacha20poly1305::{ChaCha20Poly1305, KeyInit};
use rand::random;
use serde::Deserialize;
use sqlx::PgPool;
use std::time::Instant;

pub struct UserRepository {
    db_pool: PgPool,
}
#[derive(Deserialize)]
pub struct LoginCredentials {
    pub username: String,
    pub password: String,
}
impl UserRepository {
    pub fn new(db_pool: PgPool) -> Self {
        UserRepository { db_pool }
    }

    pub async fn authenticate_user(&self, user: &LoginCredentials) -> Result<User, String> {
        let time = Instant::now();

        let res = sqlx::query!(r#"select  id, username, role as "role:UserRole", password_hash, master_key_salt, content_key_salt, content_key_encr
                                from users where username = ($1)"#, user.username)
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|_| "Database connection failed")?
            .ok_or("Couldn't find this user")?;

        Argon2::default()
            .verify_password(
                user.password.as_bytes(),
                &PasswordHash::new(&res.password_hash)
                    .map_err(|_| "Invalid password_hash from database")?,
            )
            .map_err(|_| "Invalid credentials")?;

        let master_key = Self::get_master_key_with_salt(&user.password, &res.master_key_salt);

        let content_key =
            Self::decrypt_content_key(&master_key, &res.content_key_encr, &res.content_key_salt);

        let user = User::new(
            res.id,
            res.username,
            res.role,
            content_key?,
            res.master_key_salt,
        );
        Ok(user)
    }

    pub async fn add_user(&self, user: &LoginCredentials, role: UserRole) -> Result<User, &str> {
        let (master_key, master_salt) = Self::get_master_key_and_salt(&user.password);
        let (content_key, content_cypher, content_salt) =
            Self::get_content_key_cypher_and_salt(&master_key);
        let id = sqlx::query!(
            "insert into users (username,role,password_hash,master_key_salt,content_key_encr,content_key_salt)
            values ($1,$2,$3,$4,$5,$6)
            returning id",
            user.username,
            &role as &UserRole,
            Self::hash_password(&user.password),
            &master_salt,
            &content_cypher,
            &content_salt,
        )
            .fetch_one(&self.db_pool)
            .await
            .map_err(|_| "This username is taken")?.id;

        Ok(User::new(
            id,
            user.username.clone(),
            role,
            content_key.into(),
            master_salt.into(),
        ))
    }

    fn hash_password(password: &str) -> String {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string()
    }

    fn get_master_key_and_salt(password: &str) -> ([u8; 32], [u8; 12]) {
        let salt: [u8; 12] = random();
        let mut master_key = [0u8; 32];
        Argon2::default()
            .hash_password_into(password.as_bytes(), &salt, &mut master_key)
            .unwrap();
        (master_key, salt)
    }

    fn decrypt_content_key(
        master_key: &[u8; 32],
        cyphertext: &[u8],
        salt: &[u8],
    ) -> Result<Vec<u8>, String> {
        let cypher = ChaCha20Poly1305::new(master_key.into())
            .decrypt(salt.into(), cyphertext)
            .map_err(|e| format!("Decryption failed, the key is probably outdated: {:?}", e))?;
        Ok(cypher)
    }

    fn get_master_key_with_salt(password: &str, salt: &[u8]) -> [u8; 32] {
        let mut master_key = [0u8; 32];
        Argon2::default()
            .hash_password_into(password.as_bytes(), &salt, &mut master_key)
            .unwrap();
        master_key
    }

    fn get_content_key_cypher_and_salt(master_key: &[u8; 32]) -> ([u8; 32], Vec<u8>, [u8; 12]) {
        let content_key: [u8; 32] = random();
        let salt: [u8; 12] = random();
        let cypher = ChaCha20Poly1305::new(master_key.into())
            .encrypt(&salt.into(), content_key.as_slice())
            .unwrap();
        (content_key, cypher, salt)
    }
}
