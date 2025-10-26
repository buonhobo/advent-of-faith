use crate::model::app_state::AppState;
use crate::model::user::User;
use crate::templates::calendar_templates::{CreateCalendarTemplate, ShowCalendarTemplate};
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Form;
use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CalendarCreationRequest {
    title: String,
}
pub async fn create_calendar_post(
    user: User,
    State(state): State<AppState>,
    Form(req): Form<CalendarCreationRequest>,
) -> impl IntoResponse {
    let result = state
        .calendar_service
        .create_calendar(&user, &req.title)
        .await;

    let calendar_id = result.map(|calendar| calendar.id);
    match calendar_id {
        Ok(calendar_id) => Redirect::to(&format!("/calendar/{calendar_id}")).into_response(),
        Err(e) => Html(CreateCalendarTemplate::new(Some(e)).render().unwrap()).into_response(),
    }
}

pub async fn create_calendar_get() -> impl IntoResponse {
    let content = CreateCalendarTemplate::new(None)
        .render()
        .map_err(|err| format!("There was an error rendering this page {err}"));
    Html(content)
}

#[derive(Deserialize)]
pub struct AddDayForm {
    unlocks_at: DateTime<Utc>,
}
pub async fn add_day_post(
    State(state): State<AppState>,
    Path(calendar_id): Path<i32>,
    user: User,
    Form(add_day_form): Form<AddDayForm>,
) -> impl IntoResponse {
    state
        .calendar_service
        .add_day(&user, calendar_id, add_day_form.unlocks_at)
        .await
        .expect("asda");
    Redirect::to(&format!("/calendar/{calendar_id}"))
}

pub async fn show_calendar(
    Path(calendar_id): Path<i32>,
    user: User,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let res = state
        .calendar_service
        .get_calendar_with_days(calendar_id, &user)
        .await;

    let cal = match res {
        Ok(cal) => cal,
        Err(e) => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(e)
                .unwrap()
                .into_response();
        }
    };

    let content = ShowCalendarTemplate::new(cal, user).render().unwrap();

    Html(content).into_response()
}
