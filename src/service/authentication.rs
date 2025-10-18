use crate::domain::user::User;
use crate::AppState;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::extract::CookieJar;
use base64::Engine;
use rand::RngCore;
use std::collections::HashMap;

pub struct SessionStore {
    token_to_user: HashMap<String, User>,
}
impl SessionStore {
    pub fn new() -> Self {
        Self {
            token_to_user: HashMap::new(),
        }
    }

    pub fn add_user(&mut self, user: User) -> String {
        let token = Self::generate_token();
        self.token_to_user.insert(token.clone(), user);
        token
    }

    pub fn get_user(&self, token: &str) -> Option<User> {
        self.token_to_user.get(token).cloned()
    }

    fn generate_token() -> String {
        let mut bytes = [0u8; 32]; // 256-bit random token
        rand::rng().fill_bytes(&mut bytes);
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    }
}

pub async fn authenticate(
    State(repo): State<AppState>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let user = if let Some(cookie) = jar.get("token") {
        repo.session_store.read().await.get_user(cookie.value())
    } else {
        None
    };
    req.extensions_mut().insert(user);
    Ok(next.run(req).await)
}
