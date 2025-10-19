use crate::model::user::{User, UserRole};
use crate::AppState;
use axum::extract::OptionalFromRequestParts;
use axum::extract::{Request, State};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::collections::HashMap;
use std::convert::Infallible;
use std::ops::Not;
use uuid::Uuid;

struct SessionRow {
    id: Uuid,
    user_id: i32,
    username: String,
    role: UserRole,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

impl Into<Session> for SessionRow {
    fn into(self) -> Session {
        Session {
            id: self.id,
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
    id: Uuid,
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
    cached_sessions: HashMap<Uuid, Session>,
}

impl SessionStore {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            db_pool,
            cached_sessions: HashMap::new(),
        }
    }

    async fn get_session(&self, token: &str) -> Option<Session> {
        let uuid = Uuid::parse_str(token).ok()?;
        if let Some(session) = self.cached_sessions.get(&uuid) {
            Some(session.clone())
        } else {
            sqlx::query_as!(
                SessionRow,
                r#"
                SELECT
                    u.id as user_id, u.username, u.role as "role:_",
                    s.id, s.created_at, s.expires_at
                FROM user_sessions as s
                JOIN users as u ON u.id = s.user_id
                WHERE s.id = $1
                "#,
                uuid
            )
            .fetch_optional(&self.db_pool)
            .await
            .ok()?
            .map(SessionRow::into)
        }
    }

    pub async fn get_user(&self, token: &str) -> Option<User> {
        self.get_session(token)
            .await
            .and_then(|session| session.is_expired().not().then_some(session.user))
    }

    pub async fn add_user(&mut self, user: User) -> Result<Uuid, String> {
        let session: Session = sqlx::query_as!(
            SessionRow,
            r#"
            WITH inserted AS (
                INSERT INTO user_sessions (user_id) VALUES ($1)
                RETURNING id, user_id, created_at, expires_at)
            SELECT
                u.id AS user_id, u.username, u.role AS "role:_",
                s.id, s.created_at, s.expires_at
            FROM inserted AS s
            JOIN users AS u ON u.id = s.user_id
            "#,
            user.id
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|_| "Error from database when making a new session")
        .map(SessionRow::into)?;

        let uuid = session.id;
        self.cached_sessions.insert(session.id, session);
        Ok(uuid)
    }
}

impl<S> OptionalFromRequestParts<S> for User
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        Ok(parts.extensions.get::<Option<User>>().cloned().flatten())
    }
}

pub async fn require_logged_out(
    user: Option<User>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if user.is_none() {
        Ok(next.run(request).await)
    } else {
        Ok(Redirect::to("/").into_response())
    }
}

pub async fn authenticate_user(
    State(state): State<AppState>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if let Some(token) = jar.get("token").map(Cookie::value) {
        if let Some(user) = state.session_store.read().await.get_user(token).await {
            req.extensions_mut().insert(Some(user));
        };
    };

    Ok(next.run(req).await)
}
