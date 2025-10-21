use crate::model::user::User;
use crate::templates::templates::HomeTemplate;
use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};

pub async fn welcome_handler(maybe_user: Option<User>) -> Result<impl IntoResponse, StatusCode> {
    Ok(Html(
        HomeTemplate::with_user(maybe_user)
            .render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}
