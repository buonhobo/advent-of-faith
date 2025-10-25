use crate::model::app_state::AppState;
use crate::model::user::User;
use crate::templates::templates::HelloTemplate;
use askama::Template;
use axum::extract::State;
use axum::response::{Html, IntoResponse};

pub async fn dashboard_handler(user: User, State(state): State<AppState>) -> impl IntoResponse {
    let subscriptions = state.calendar_service.get_dashboard_data(&user).await;
    let content = match subscriptions {
        Ok(subscriptions) => HelloTemplate::new(user, subscriptions)
            .render()
            .map_err(|_| "There was an error rendering this page".to_owned()),
        Err(e) => Err(format!("There was an error getting your dashboard: {e}")),
    };

    match content {
        Ok(html) => Html(html),
        Err(msg) => Html(msg),
    }
}
