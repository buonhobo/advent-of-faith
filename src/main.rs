mod domain;
mod persistence;
mod service;
mod web;

use crate::persistence::repository::UserRepository;
use crate::service::authentication::SessionStore;
use crate::web::handler::{login_page, login_post, signup_page, signup_post};
use axum::{routing::get, Router};
use sqlx::PgPool;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;
use web::handler::web_handler;

#[derive(Clone)]
struct AppState {
    pub user_repository: Arc<RwLock<UserRepository>>,
    pub session_store: Arc<RwLock<SessionStore>>,
}

impl AppState {
    async fn new(db_conn: PgPool) -> Self {
        Self {
            user_repository: Arc::new(RwLock::new(UserRepository::new(db_conn.clone()))),
            session_store: Arc::new(RwLock::new(SessionStore::new(db_conn))),
        }
    }
}

#[tokio::main]
async fn main() {
    let db_conn = PgPool::connect(&env::var("DATABASE_URL").expect("DATABASE_URL not set!"))
        .await
        .expect("DATABASE connection failed!");
    let state: AppState = AppState::new(db_conn).await;

    // build our application with a single route
    let app = Router::new()
        .route("/", get(web_handler))
        .route("/login", get(login_page).post(login_post))
        .route("/signup", get(signup_page).post(signup_post))
        .with_state(state.clone());

    // run our app with hyper, listening globally on port 8080
    let listener =
        tokio::net::TcpListener::bind(&env::var("BIND_LISTENER").expect("BIND_LISTENER not set!"))
            .await
            .unwrap();
    axum::serve(listener, app).await.unwrap();
}
