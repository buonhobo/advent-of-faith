use crate::model::app_state::AppState;
use crate::model::user::User;
use crate::templates::calendar_templates::{
    CreateCalendarTemplate, ShowCalendarTemplate, ShowDayTemplate, UnlockDayTemplate,
};
use askama::Template;
use axum::extract::{Path, Query, State};
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

pub async fn subscribe_post(
    user: User,
    Path(calendar_id): Path<i32>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let result = state.calendar_service.subscribe(&user, calendar_id).await;

    match result {
        Ok(()) => Redirect::to(&format!("/calendar/{calendar_id}")).into_response(),
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
    password: Option<String>,
    content:String,
}
pub async fn add_day_post(
    State(state): State<AppState>,
    Path(calendar_id): Path<i32>,
    user: User,
    Form(add_day_form): Form<AddDayForm>,
) -> impl IntoResponse {
    state
        .calendar_service
        .add_day(&user, calendar_id, add_day_form.unlocks_at, add_day_form.password, add_day_form.content)
        .await
        .expect("asda");
    Redirect::to(&format!("/calendar/{calendar_id}"))
}


//TODO add subscription button if user is not subscribed
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

//TODO: if user has not unlocked the day, redirect to day unlock page
pub async fn show_day_get(
    Path((cal_id, day_id)): Path<(i32, i32)>,
    user: User,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let res = state.calendar_service.get_rich_content(day_id, &user).await;

    let content = match res {
        Ok(cal) => cal,
        Err(e) => {
            return Redirect::to(&format!("/calendar/{cal_id}/day/{day_id}/unlock")).into_response();
        }
    };

    let content = ShowDayTemplate::new(content, user).render().unwrap();

    Html(content).into_response()
}
#[derive(Deserialize)]
pub struct UnlockDayForm {
    code: Option<String>,
}
pub async fn unlock_post(
    State(state): State<AppState>,
    Path((calendar_id, day_id)): Path<(i32, i32)>,
    user: User,
    Form(unlock_form): Form<UnlockDayForm>,
) -> Result<Response, Response> {
    let res = state
        .calendar_service
        .unlock_day(day_id, &user, unlock_form.code.clone())
        .await;

    let output = match res {
        Ok(_) => Redirect::to(&format!("/calendar/{calendar_id}/day/{day_id}")).into_response(),
        Err(e) => Html(
            UnlockDayTemplate::new(unlock_form.code, user, day_id,calendar_id)
                .with_message(Some(e))
                .render()
                .map_err(|e| {
                    Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .body(e.to_string())
                        .unwrap()
                        .into_response()
                })?,
        )
        .into_response(),
    };

    Ok(output)
}


//TODO: If user has already unlocked the day, then redirect to the day
//TODO: Fetch day unlock date
//TODO: If day is not protected, don't ask for password
pub async fn unlock_get(
    Path((cal_id, day_id)): Path<(i32, i32)>,
    user: User,
    Query(unlock_form): Query<UnlockDayForm>,
) -> impl IntoResponse {
    let content = UnlockDayTemplate::new(unlock_form.code, user, day_id, cal_id)
        .render()
        .unwrap();
    Html(content).into_response()
}
