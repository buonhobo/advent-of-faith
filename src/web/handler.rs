use crate::domain::user::UserRole;
use crate::service::authentication::CurrentUser;
use crate::web::templates::{HelloTemplate, HomeTemplate, LoginTemplate, SignupTemplate};
use crate::AppState;
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Form;
use axum_extra::extract::cookie::{Cookie, CookieJar, Expiration, SameSite};
use serde::Deserialize;

pub async fn web_handler(CurrentUser(user): CurrentUser) -> Result<impl IntoResponse, StatusCode> {
    if let Some(user) = user {
        Ok(Html(
            HelloTemplate::new(user)
                .render()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ))
    } else {
        Ok(Html(
            HomeTemplate
                .render()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ))
    }
}

pub async fn login_page(CurrentUser(user): CurrentUser) -> Result<impl IntoResponse, StatusCode> {
    if user.is_some() {
        return Ok(Redirect::to("/").into_response());
    }

    LoginTemplate::empty()
        .render()
        .map(|v| Html(v).into_response())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Deserialize, Debug)]
pub struct LoginCredentials {
    pub username: String,
    pub password: String,
}

pub async fn login_post(
    State(state): State<AppState>,
    Form(login): Form<LoginCredentials>,
) -> Result<(CookieJar, Response), StatusCode> {
    let mut cookie_jar = CookieJar::new();
    let response;

    let user = state
        .user_repository
        .read()
        .await
        .authenticate_user(&login)
        .await
        .ok();

    if let Some(user) = user {
        response = Redirect::to("/").into_response();
        let token = state
            .session_store
            .write()
            .await
            .add_user(user)
            .await
            .unwrap();
        cookie_jar = cookie_jar.add(get_cookie(token.to_string()));
    } else {
        response = LoginTemplate::with_message("Invalid username or password".to_owned(), login)
            .render()
            .map(|v| Html(v).into_response())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }

    Ok((cookie_jar, response))
}

pub async fn signup_page(CurrentUser(user): CurrentUser) -> Result<Response, StatusCode> {
    if user.is_some() {
        return Ok(Redirect::to("/").into_response());
    }

    SignupTemplate::empty()
        .render()
        .map(|v| Html(v).into_response())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn signup_post(
    State(state): State<AppState>,
    Form(login): Form<LoginCredentials>,
) -> Result<(CookieJar, Response), StatusCode> {
    let user = state
        .user_repository
        .write()
        .await
        .add_user(&login, UserRole::MEMBER)
        .await;

    let mut cookie_jar = CookieJar::new();
    let redirect;

    match user {
        Ok(user) => {
            let token = state
                .session_store
                .write()
                .await
                .add_user(user)
                .await
                .unwrap();
            cookie_jar = cookie_jar.add(get_cookie(token.to_string()));
            redirect = Redirect::to("/").into_response();
        }
        Err(message) => {
            redirect = SignupTemplate::with_message(message, login)
                .render()
                .map(|v| Html(v).into_response())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    }

    Ok((cookie_jar, redirect))
}

fn get_cookie(token: String) -> Cookie<'static> {
    Cookie::build(("token", token))
        .secure(true)
        .http_only(true)
        .expires(Expiration::Session)
        .same_site(SameSite::Strict)
        .build()
}
