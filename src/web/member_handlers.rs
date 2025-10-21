use crate::model::calendar::{Day, Status};
use crate::model::user::User;
use crate::templates::templates::HelloTemplate;
use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};

pub async fn dashboard_handler(user: User) -> Result<impl IntoResponse, StatusCode> {
    Ok(Html(
        HelloTemplate::new(
            user,
            vec![
                Day {
                    number: 1,
                    status: Status::Unlocked,
                },
                Day {
                    number: 2,
                    status: Status::Locked,
                },
                Day {
                    number: 3,
                    status: Status::Future,
                },
            ],
        )
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}
