use crate::model::app_state::AppState;
use crate::model::calendar::{UserCalendar, UserDay};
use crate::model::user::User;
use crate::templates::calendar_templates::{
    CreateCalendarTemplate, ShowCalendarTemplate, ShowDayTemplate, UnlockDayTemplate,
};
use askama::Template;
use axum::extract::{Query, State};
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
    user_calendar: UserCalendar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let result = state
        .calendar_service
        .subscribe(&user, &user_calendar)
        .await;

    match result {
        Ok(()) => Redirect::to(&format!("/calendar/{}", user_calendar.calendar.id)).into_response(),
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
    content: String,
}
pub async fn add_day_post(
    State(state): State<AppState>,
    user_calendar: UserCalendar,
    user: User,
    Form(add_day_form): Form<AddDayForm>,
) -> impl IntoResponse {
    state
        .calendar_service
        .add_day(
            &user,
            &user_calendar,
            add_day_form.unlocks_at,
            add_day_form.password,
            add_day_form.content,
        )
        .await
        .expect("asda");
    Redirect::to(&format!("/calendar/{}", user_calendar.calendar.id))
}

pub async fn show_calendar(
    user_calendar: UserCalendar,
    user: User,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let res = state
        .calendar_service
        .get_calendar_user_days(&user_calendar, &user)
        .await;

    let days = match res {
        Ok(days) => days,
        Err(e) => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(e)
                .unwrap()
                .into_response();
        }
    };

    let content = ShowCalendarTemplate::new(user_calendar, days, user)
        .render()
        .unwrap();

    Html(content).into_response()
}

pub async fn show_day_get(
    user_calendar: UserCalendar,
    user_day: UserDay,
    user: User,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let res = state
        .calendar_service
        .get_rich_content(&user_day, &user)
        .await;

    let content = match res {
        Ok(cal) => cal,
        Err(_) => {
            return Redirect::to(&format!(
                "/calendar/{}/day/{}/unlock",
                user_calendar.calendar.id, user_day.day.id
            ))
            .into_response();
        }
    };

    let content = ShowDayTemplate::new(user_day, user_calendar, content, user)
        .render()
        .unwrap();

    Html(content).into_response()
}
#[derive(Deserialize)]
pub struct UnlockDayForm {
    code: Option<String>,
}
pub async fn unlock_post(
    State(state): State<AppState>,
    user_calendar: UserCalendar,
    user_day: UserDay,
    user: User,
    Form(unlock_form): Form<UnlockDayForm>,
) -> Result<Response, Response> {
    let res = state
        .calendar_service
        .unlock_day(&user_day, &user, unlock_form.code.clone())
        .await;

    let output = match res {
        Ok(_) => Redirect::to(&format!(
            "/calendar/{}/day/{}",
            user_calendar.calendar.id, user_day.day.id
        ))
        .into_response(),
        Err(e) => Html(
            UnlockDayTemplate::new(unlock_form.code, user_day)
                .with_message(e)
                .render()
                .unwrap(),
        )
        .into_response(),
    };

    Ok(output)
}

pub async fn unlock_get(
    user_calendar: UserCalendar,
    user_day: UserDay,
    Query(unlock_form): Query<UnlockDayForm>,
) -> impl IntoResponse {
    if user_day.unlocked_at.is_some() {
        return Redirect::to(&format!(
            "/calendar/{}/day/{}",
            user_calendar.calendar.id, user_day.day.id
        ))
        .into_response();
    }

    let content = UnlockDayTemplate::new(unlock_form.code, user_day)
        .render()
        .unwrap();
    Html(content).into_response()
}

pub async fn delete_day_post(
    State(state): State<AppState>,
    user_calendar: UserCalendar,
    user_day: UserDay,
    user: User,
) -> Result<Response, Response> {
    let res = state
        .calendar_service
        .delete_day(&user_calendar, &user_day, &user)
        .await;

    let output = match res {
        Ok(_) => Redirect::to(&format!("/calendar/{}", user_calendar.calendar.id)).into_response(),
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(e)
            .unwrap()
            .into_response(),
    };

    Ok(output)
}
