use crate::model::app_state::AppState;
use crate::model::user::{User, UserRole};
use crate::persistence::user_repository::LoginCredentials;
use crate::templates::authentication_templates::{
    ChangePassTemplate, LoginTemplate, SignupTemplate,
};
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Form;
use axum_extra::extract::cookie::{Cookie, Expiration, SameSite};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use uuid::Uuid;

pub async fn login_page() -> Result<impl IntoResponse, StatusCode> {
    LoginTemplate::empty()
        .render()
        .map(|v| Html(v).into_response())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Deserialize, Clone)]
pub struct LoginForm {
    username: String,
    password: String,
}
impl Into<LoginCredentials> for LoginForm {
    fn into(self) -> LoginCredentials {
        LoginCredentials {
            username: self.username,
            password: self.password,
        }
    }
}

pub async fn login_post(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(login): Form<LoginForm>,
) -> Result<(CookieJar, Response), StatusCode> {
    let credentials = login.clone().into();
    let user_repo_lock = state.user_repository.read().await;
    let user = user_repo_lock.authenticate_user(&credentials).await;

    Ok(match user {
        Ok(user) => {
            let token = state
                .session_store
                .write()
                .await
                .add_user(user, &login.password)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let (jar, target) = match jar.get("next") {
                Some(next) => (jar.clone().remove(next.clone()), next.value()),
                None => (jar, "/"),
            };
            let jar = jar.add(get_cookie(token.to_string()));
            let response = Redirect::to(target).into_response();
            (jar, response)
        }
        Err(msg) => {
            let response = LoginTemplate::with_message(msg.to_owned(), credentials)
                .render()
                .map(|v| Html(v).into_response())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR.into_response());
            (jar, response)
        }
    })
}

pub async fn signup_page() -> Result<Response, StatusCode> {
    SignupTemplate::empty()
        .render()
        .map(|v| Html(v).into_response())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn signup_post(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(login): Form<LoginForm>,
) -> Result<(CookieJar, Response), StatusCode> {
    let credentials = login.clone().into();
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
                .add_user(user, &login.password)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let (jar, target) = match jar.get("next") {
                Some(next) => (jar.clone().remove(next.clone()), next.value()),
                None => (jar, "/"),
            };
            let jar = jar.add(get_cookie(token.to_string()));
            let response = Redirect::to(target).into_response();
            (jar, response)
        }
        Err(message) => {
            let redirect = SignupTemplate::with_message(message.to_owned(), credentials)
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

pub async fn change_pass_get() -> Result<Response, StatusCode> {
    ChangePassTemplate::empty()
        .render()
        .map(|v| Html(v).into_response())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Deserialize, Clone)]
pub struct ChangePasswordForm {
    old_password: String,
    new_password: String,
}
pub async fn change_pass_post(
    State(state): State<AppState>,
    jar: CookieJar,
    user: User,
    Form(form): Form<ChangePasswordForm>,
) -> Result<Response, StatusCode> {
    let res: Result<(), String> = state
        .user_repository
        .read()
        .await
        .change_password(&user, &form.old_password, &form.new_password)
        .await;

    let session_id = jar.get("token").ok_or(StatusCode::BAD_REQUEST)?.value();
    let uuid = Uuid::parse_str(&session_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    state
        .session_store
        .write()
        .await
        .expire_session(uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match res {
        Ok(()) => Ok(Redirect::to("/").into_response()),
        Err(message) => ChangePassTemplate::with_message(message)
            .render()
            .map(|v| Html(v).into_response())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR),
    }
}
