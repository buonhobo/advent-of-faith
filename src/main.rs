mod model;
mod persistence;
mod service;
mod templates;
mod web;

use crate::model::app_state::AppState;
use crate::service::authentication::{authenticate_user, require_logged_in, require_logged_out};
use crate::web::authentication_handlers::{
    login_page, login_post, logout_get, signup_page, signup_post,
};
use crate::web::calendar_handlers::{
    add_day_post, create_calendar_get, create_calendar_post, show_calendar,
};
use crate::web::handler::welcome_handler;
use crate::web::member_handlers::dashboard_handler;
use axum::routing::{get_service, post};
use axum::{middleware, routing::get, Router};
use sqlx::PgPool;
use std::env;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let db_conn = PgPool::connect(&env::var("DATABASE_URL").expect("DATABASE_URL not set!"))
        .await
        .expect("DATABASE connection failed!");
    let state: AppState = AppState::new(&db_conn).await;

    let login_router = Router::new()
        .route("/login", get(login_page).post(login_post))
        .route("/signup", get(signup_page).post(signup_post))
        .route_layer(middleware::from_fn(require_logged_out));
    let guest_router = Router::new()
        .route("/", get(welcome_handler))
        .nest_service("/static", get_service(ServeDir::new("static")));

    let day_router = Router::new().route("/create", post(add_day_post));

    let calendar_router = Router::new()
        .route(
            "/create",
            get(create_calendar_get).post(create_calendar_post),
        )
        .route("/{calendar_id}", get(show_calendar))
        .nest("/{calendar_id}/day", day_router);

    let user_router = Router::new()
        .route("/home", get(dashboard_handler))
        .route("/logout", get(logout_get))
        .nest("/calendar", calendar_router)
        .route_layer(middleware::from_fn(require_logged_in));

    // build our application with a single route
    let app = Router::new()
        .merge(login_router)
        .merge(guest_router)
        .merge(user_router)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            authenticate_user,
        ))
        .with_state(state.clone());

    // run our app with hyper, listening globally on port 8080
    let listener =
        tokio::net::TcpListener::bind(&env::var("BIND_LISTENER").expect("BIND_LISTENER not set!"))
            .await
            .unwrap();
    axum::serve(listener, app).await.unwrap();
}
