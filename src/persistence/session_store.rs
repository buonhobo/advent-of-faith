use crate::model::user::{User, UserRole};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::collections::HashMap;
use std::ops::Not;
use uuid::Uuid;

struct SessionRow {
    token_hash: String,
    user_id: i32,
    username: String,
    role: UserRole,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

impl Into<Session> for SessionRow {
    fn into(self) -> Session {
        Session {
            id: hex::decode(self.token_hash)
                .expect("Invalid hex in database")
                .try_into()
                .expect("Invalid hex in database"),
            created_at: self.created_at,
            expires_at: self.expires_at,
            user: User {
                id: self.user_id,
                username: self.username,
                role: self.role,
            },
        }
    }
}

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
    cached_sessions: HashMap<[u8; 32], Session>,
}

impl SessionStore {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            db_pool,
            cached_sessions: HashMap::new(),
        }
    }

    async fn get_session(&mut self, token: Uuid) -> Option<Session> {
        let token_hash: [u8; 32] = Sha256::digest(token).into();
        let hash_hex = hex::encode(token_hash);
        if let Some(session) = self.cached_sessions.get(&token_hash) {
            Some(session.clone())
        } else {
            println!("Querying DB for session with hash: {}", hash_hex);
            let result = sqlx::query_as!(
                SessionRow,
                r#"
                SELECT
                    u.id as user_id, u.username, u.role as "role:_",
                    s.token_hash, s.created_at, s.expires_at
                FROM user_sessions as s
                JOIN users as u ON u.id = s.user_id
                WHERE s.token_hash = $1
                "#,
                hash_hex
            )
            .fetch_optional(&self.db_pool)
            .await;
            let session: Session = result.ok()?.map(SessionRow::into)?;
            self.cached_sessions.insert(token_hash, session.clone());
            Some(session)
        }
    }

    pub async fn get_user(&mut self, token: Uuid) -> Option<User> {
        self.get_session(token)
            .await
            .and_then(|session| session.is_expired().not().then_some(session.user))
    }

    pub async fn add_user(&mut self, user: User) -> Result<Uuid, String> {
        loop {
            let candidate = Uuid::new_v4();
            let token_hash: [u8; 32] = Sha256::digest(candidate).into();
            let hash_hex = hex::encode(token_hash);
            println!("Querying DB to make session with hash: {}", hash_hex);
            let result = sqlx::query_as!(
                SessionRow,
                r#"
                WITH inserted AS (
                    INSERT INTO user_sessions (token_hash,user_id) VALUES ($1,$2)
                    RETURNING token_hash, user_id, created_at, expires_at)
                SELECT
                    u.id AS user_id, u.username, u.role AS "role:_",
                    s.token_hash, s.created_at, s.expires_at
                FROM inserted AS s
                JOIN users AS u ON u.id = s.user_id
                "#,
                hash_hex,
                user.id
            )
            .fetch_one(&self.db_pool)
            .await;

            match result {
                Ok(session) => {
                    let session: Session = session.into();
                    self.cached_sessions.insert(token_hash, session);
                    return Ok(candidate);
                }
                Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                    continue;
                }
                Err(e) => {
                    return Err(e.to_string());
                }
            }
        }
    }

    pub async fn expire_session(&mut self, session_id: Uuid) -> Result<(), String> {
        let token_hash: [u8; 32] = Sha256::digest(session_id.as_bytes()).into();
        let hash_hex = hex::encode(token_hash);
        println!("Querying DB to expire session with hash: {}", hash_hex);
        sqlx::query!(
            "UPDATE user_sessions SET expires_at = now() WHERE token_hash = $1",
            hash_hex
        )
        .execute(&self.db_pool)
        .await
        .map_err(|_| "Error from database when expiring session")?;
        self.cached_sessions.remove(&token_hash);
        Ok(())
    }
}
