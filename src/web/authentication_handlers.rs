use crate::model::app_state::AppState;
use crate::model::user::UserRole;
use crate::persistence::user_repository::LoginCredentials;
use crate::templates::authentication_templates::{LoginTemplate, NextResource, SignupTemplate};
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Form;
use axum_extra::extract::cookie::{Cookie, Expiration, SameSite};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use uuid::Uuid;

pub async fn login_page(Query(next): Query<NextResource>) -> Result<impl IntoResponse, StatusCode> {
    LoginTemplate::empty()
        .with_next(next)
        .render()
        .map(|v| Html(v).into_response())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Deserialize, Clone)]
pub struct LoginForm {
    username: String,
    password: String,
    next: Option<String>,
}
impl Into<LoginCredentials> for LoginForm {
    fn into(self) -> LoginCredentials {
        LoginCredentials {
            username: self.username,
            password: self.password,
        }
    }
}

impl Into<NextResource> for LoginForm {
    fn into(self) -> NextResource {
        NextResource { next: self.next }
    }
}

pub async fn login_post(
    State(state): State<AppState>,
    Form(login): Form<LoginForm>,
) -> Result<(CookieJar, Response), StatusCode> {
    let next: NextResource = login.clone().into();
    let credentials = login.into();
    let user_repo_lock = state.user_repository.read().await;
    let user = user_repo_lock.authenticate_user(&credentials).await;

    Ok(match user {
        Ok(user) => {
            let response = Redirect::to(&next.get_next_or("/")).into_response();
            let token = state
                .session_store
                .write()
                .await
                .add_user(user)
                .await
                .unwrap();
            let cookie_jar = CookieJar::new().add(get_cookie(token.to_string()));
            (cookie_jar, response)
        }
        Err(msg) => {
            let response = LoginTemplate::with_message(msg.to_owned(), credentials)
                .with_next(next)
                .render()
                .map(|v| Html(v).into_response())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR.into_response());
            let cookie_jar = CookieJar::new();
            (cookie_jar, response)
        }
    })
}

pub async fn signup_page(Query(next): Query<NextResource>) -> Result<Response, StatusCode> {
    SignupTemplate::empty()
        .with_next(next)
        .render()
        .map(|v| Html(v).into_response())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn signup_post(
    State(state): State<AppState>,
    Form(login): Form<LoginForm>,
) -> Result<(CookieJar, Response), StatusCode> {
    let next: NextResource = login.clone().into();
    let credentials = login.into();
    let user_repo_lock = state.user_repository.write().await;
    let user = user_repo_lock
        .add_user(&credentials, UserRole::MEMBER)
        .await;

    Ok(match user {
        Ok(user) => {
            let token = state
                .session_store
                .write()
                .await
                .add_user(user)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let redirect = Redirect::to(&next.get_next_or("/")).into_response();
            let cookie_jar = CookieJar::new().add(get_cookie(token.to_string()));
            (cookie_jar, redirect)
        }
        Err(message) => {
            let redirect = SignupTemplate::with_message(message.to_owned(), credentials)
                .with_next(next)
                .render()
                .map(|v| Html(v).into_response())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR.into_response());

            let cookie_jar = CookieJar::new();
            (cookie_jar, redirect)
        }
    })
}

pub async fn logout_get(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), StatusCode> {
    let session_id = jar.get("token").ok_or(StatusCode::BAD_REQUEST)?.value();
    let uuid = Uuid::parse_str(&session_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let jar = jar.remove(Cookie::from("token"));

    state
        .session_store
        .write()
        .await
        .expire_session(uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((jar, Redirect::to("/")))
}

fn get_cookie(token: String) -> Cookie<'static> {
    Cookie::build(("token", token))
        .secure(true)
        .http_only(true)
        .expires(Expiration::Session)
        .same_site(SameSite::Strict)
        .build()
}
