use crate::model::user::User;
use crate::AppState;
use askama::filters::urlencode;
use axum::extract::{FromRequestParts, OptionalFromRequestParts};
use axum::extract::{Request, State};
use axum::http::request::Parts;
use axum::http::uri::PathAndQuery;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use std::convert::Infallible;
use uuid::Uuid;

impl<S> OptionalFromRequestParts<S> for User
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Option<Self>, Self::Rejection> {
        Ok(parts.extensions.get::<Option<User>>().cloned().flatten())
    }
}

impl<S> FromRequestParts<S> for User
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(parts
            .extensions
            .get::<Option<User>>()
            .cloned()
            .flatten()
            .ok_or(StatusCode::UNAUTHORIZED)?)
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

pub async fn require_logged_in(
    user: Option<User>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if user.is_some() {
        Ok(next.run(request).await)
    } else {
        let target = request
            .uri()
            .path_and_query()
            .map_or("/", PathAndQuery::as_str);
        Ok(Redirect::to(&format!("/login?next={}", urlencode(target).unwrap())).into_response())
    }
}

pub async fn authenticate_user(
    State(state): State<AppState>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if let Some(token) = jar.get("token").map(Cookie::value) {
        let token = Uuid::parse_str(&token).map_err(|_| StatusCode::BAD_REQUEST)?;
        if let Some(user) = state.session_store.write().await.get_user(token).await {
            req.extensions_mut().insert(Some(user));
        };
    };

    Ok(next.run(req).await)
}
