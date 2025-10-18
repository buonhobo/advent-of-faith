use crate::domain::user::{User, UserRole};
use crate::web::templates::{HelloTemplate, HomeTemplate, LoginTemplate, SignupTemplate};
use crate::AppState;
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::{Extension, Form};
use axum_extra::extract::cookie::{Cookie, CookieJar, Expiration, SameSite};
use serde::Deserialize;

pub async fn web_handler(
    Extension(user): Extension<Option<User>>,
) -> Result<impl IntoResponse, StatusCode> {
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

pub async fn login_page(
    Extension(user): Extension<Option<User>>,
) -> Result<impl IntoResponse, StatusCode> {
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
    State(repo): State<AppState>,
    Form(login): Form<LoginCredentials>,
) -> Result<(CookieJar, Response), StatusCode> {
    let mut cookie_jar = CookieJar::new();
    let response;

    let user = repo
        .user_repository
        .read()
        .await
        .authenticate_user(&login)
        .await
        .ok();

    if let Some(user) = user {
        response = Redirect::to("/").into_response();
        let token = repo.session_store.write().await.add_user(user);
        cookie_jar = cookie_jar.add(get_cookie(token));
    } else {
        response = LoginTemplate::with_message("Invalid username or password".to_owned(), login)
            .render()
            .map(|v| Html(v).into_response())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }

    Ok((cookie_jar, response))
}

pub async fn signup_page(Extension(user): Extension<Option<User>>) -> Result<Response, StatusCode> {
    if user.is_some() {
        return Ok(Redirect::to("/").into_response());
    }

    SignupTemplate::empty()
        .render()
        .map(|v| Html(v).into_response())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn signup_post(
    State(repo): State<AppState>,
    Form(login): Form<LoginCredentials>,
) -> Result<(CookieJar, Response), StatusCode> {
    let user = repo
        .user_repository
        .write()
        .await
        .add_user(&login, UserRole::MEMBER)
        .await;

    let mut cookie_jar = CookieJar::new();
    let redirect;

    match user {
        Ok(user) => {
            let token = repo.session_store.write().await.add_user(user);
            cookie_jar = cookie_jar.add(get_cookie(token));
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
