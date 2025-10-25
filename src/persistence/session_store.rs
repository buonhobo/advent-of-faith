use crate::model::user::{User, UserRole};
use argon2::Argon2;
use chacha20poly1305::aead::Aead;
use chacha20poly1305::ChaCha20Poly1305;
use chacha20poly1305::KeyInit;
use chrono::{DateTime, Utc};
use hkdf::Hkdf;
use lru::LruCache;
use rand::random;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::num::NonZeroUsize;
use uuid::Uuid;

#[derive(Clone)]
struct Session {
    id: [u8; 32],
    user: User,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}
impl Session {
    fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

pub struct SessionStore {
    db_pool: PgPool,
    cached_sessions: LruCache<[u8; 32], Session>,
}

impl SessionStore {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            db_pool,
            cached_sessions: LruCache::new(NonZeroUsize::new(100).unwrap()), // Can't fail because 100 > 0
        }
    }

    async fn get_session(&mut self, token: Uuid) -> Option<Session> {
        let token_hash: [u8; 32] = Sha256::digest(token).into();
        let hash_hex = hex::encode(token_hash);
        if let Some(session) = self.cached_sessions.get(&token_hash) {
            return Some(session.clone());
        }
        let result = sqlx::query!(
                r#"
                SELECT
                    u.id as user_id, u.username, u.role as "role:UserRole", u.content_key_encr, u.content_key_salt,
                    s.token_hash, s.created_at, s.expires_at, s.master_key_salt, s.master_key_encr
                FROM user_sessions as s
                JOIN users as u ON u.id = s.user_id
                WHERE s.token_hash = $1 AND s.expires_at > NOW()
                "#,
                hash_hex
            )
            .fetch_optional(&self.db_pool)
            .await;
        let session: Result<Session, String> = result.ok()?.map(|record| {
            let content_key = Self::get_content_key(
                &token,
                &record.master_key_encr,
                &record.master_key_salt,
                &record.content_key_encr,
                &record.content_key_salt,
            )?;
            let user = User::new(
                record.user_id,
                record.username,
                record.role,
                content_key,
                record.master_key_salt,
            );
            Ok(Session {
                id: hex::decode(record.token_hash)
                    .expect("Invalid hex in database")
                    .try_into()
                    .expect("Invalid hex in database"),
                user,
                created_at: record.created_at,
                expires_at: record.expires_at,
            })
        })?;
        let session = session.ok()?;
        self.cached_sessions.put(token_hash, session.clone());
        Some(session)
    }

    fn get_content_key(
        token: &Uuid,
        master_key_encr: &[u8],
        master_key_salt: &[u8],
        content_key_encr: &[u8],
        content_key_salt: &[u8],
    ) -> Result<Vec<u8>, String> {
        let mut token_key_dest = [0u8; 32];
        let hk = Hkdf::<Sha256>::new(None, token.as_bytes());
        hk.expand(b"session", &mut token_key_dest).unwrap(); // Should never fail since lengths are always the same

        let master_key = ChaCha20Poly1305::new(&token_key_dest.into())
            .decrypt(master_key_salt.into(), master_key_encr)
            .map_err(|e| e.to_string())?;
        let content_key = ChaCha20Poly1305::new(master_key.as_slice().into())
            .decrypt(content_key_salt.into(), content_key_encr)
            .map_err(|e| e.to_string())?;
        Ok(content_key)
    }

    pub async fn get_user(&mut self, token: Uuid) -> Option<User> {
        self.get_session(token).await.map(|session| session.user)
    }

    pub async fn add_user(&mut self, user: User, password: &str) -> Result<Uuid, String> {
        let token = Uuid::new_v4();
        let token_hash: [u8; 32] = Sha256::digest(token).into();
        let hash_hex = hex::encode(token_hash);

        let (master_encr, master_salt) =
            self.get_encrypted_master_key(&token, password, &user.master_key_salt);

        let result = sqlx::query!(
            r#"
                WITH inserted AS (
                    INSERT INTO user_sessions (token_hash,user_id,master_key_salt,master_key_encr)
                    VALUES ($1,$2,$3,$4)
                    RETURNING token_hash, user_id, created_at, expires_at)
                SELECT
                    s.created_at, s.expires_at
                FROM inserted AS s
                JOIN users AS u ON u.id = s.user_id
                "#,
            hash_hex,
            user.id,
            &master_salt,
            &master_encr,
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| {
            format!(
                "There was an error when creating the session: {}",
                e.to_string()
            )
        })?;

        let session = Session {
            id: token_hash,
            user,
            created_at: result.created_at,
            expires_at: result.expires_at,
        };
        self.cached_sessions.put(token_hash, session);
        Ok(token)
    }

    fn get_encrypted_master_key(
        &self,
        token: &Uuid,
        password: &str,
        master_key_salt: &[u8],
    ) -> (Vec<u8>, [u8; 12]) {
        let mut master_key = [0u8; 32];
        Argon2::default()
            .hash_password_into(password.as_bytes(), &master_key_salt, &mut master_key)
            .unwrap(); //Shouldn't fail because parameters are always of the same lengths

        let mut token_key_dest = [0u8; 32];
        let hk = Hkdf::<Sha256>::new(None, token.as_bytes());
        hk.expand(b"session", &mut token_key_dest).unwrap();

        let salt: [u8; 12] = random();
        let cypher = ChaCha20Poly1305::new(&token_key_dest.into())
            .encrypt(&salt.into(), master_key.as_slice())
            .unwrap();
        (cypher, salt)
    }

    pub async fn expire_session(&mut self, session_id: Uuid) -> Result<(), String> {
        let token_hash: [u8; 32] = Sha256::digest(session_id.as_bytes()).into();
        let hash_hex = hex::encode(token_hash);
        sqlx::query!(
            "UPDATE user_sessions SET expires_at = now() WHERE token_hash = $1",
            hash_hex
        )
        .execute(&self.db_pool)
        .await
        .map_err(|_| "Error from database when expiring session")?;
        self.cached_sessions.pop(&token_hash);
        Ok(())
    }
}
