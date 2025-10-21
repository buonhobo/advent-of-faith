use crate::persistence::session_store::SessionStore;
use crate::persistence::user_repository::UserRepository;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub user_repository: Arc<RwLock<UserRepository>>,
    pub session_store: Arc<RwLock<SessionStore>>,
}

impl AppState {
    pub async fn new(db_conn: PgPool) -> Self {
        Self {
            user_repository: Arc::new(RwLock::new(UserRepository::new(db_conn.clone()))),
            session_store: Arc::new(RwLock::new(SessionStore::new(db_conn))),
        }
    }
}
