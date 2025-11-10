use crate::model::app_state::AppState;
use crate::model::calendar::{Calendar, RichUserCalendar, UserCalendar, UserDay};
use crate::model::user::User;
use crate::persistence::calendar_repository::CalendarRepository;
use axum::extract::{FromRequestParts, Path, Request, State};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard};

#[derive(Clone)]
pub struct CalendarService {
    repo: Arc<RwLock<CalendarRepository>>,
}
impl CalendarService {
    async fn get_user_calendar(&self, cal_id: i32, user: &User) -> Result<UserCalendar, String> {
        self.get_repo().await.get_user_calendar(cal_id, user).await
    }

    async fn get_user_day(
        &self,
        user_calendar: &UserCalendar,
        day_id: i32,
        user: &User,
    ) -> Result<UserDay, String> {
        self.get_repo()
            .await
            .get_user_day_with_key(user_calendar, day_id, user)
            .await
    }
}

impl CalendarService {
    pub async fn edit_content(
        &self,
        user_calendar: &UserCalendar,
        user_day: &UserDay,
        user: &User,
        content: String,
    ) -> Result<(), String> {
        if user.id != user_calendar.calendar.owner_id {
            return Err(format!(
                "user {} cannot is not the owner of day {}",
                user.username, user_day.day.id
            ));
        }

        self.get_repo().await.edit_content(user_day, content).await
    }

    pub async fn edit_password(
        &self,
        user_calendar: &UserCalendar,
        user_day: &UserDay,
        user: &User,
        password: Option<String>,
    ) -> Result<(), String> {
        if user.id != user_calendar.calendar.owner_id {
            return Err(format!(
                "user {} cannot is not the owner of day {}",
                user.username, user_day.day.id
            ));
        }

        match password {
            Some(password) => {
                if user_day.day.protected {
                    self.get_repo()
                        .await
                        .update_password(user_day, user, &password)
                        .await
                } else {
                    self.get_repo()
                        .await
                        .set_password(user_day, user, &password)
                        .await
                }
            }
            None => {
                if user_day.day.protected {
                    self.get_repo().await.remove_password(user_day).await
                } else {
                    Ok(())
                }
            }
        }
    }

    pub async fn unlock_day(
        &self,
        user_day: &UserDay,
        user: &User,
        code: Option<String>,
    ) -> Result<(), String> {
        if user_day.is_unlocked() {
            return Err(format!(
                "day {} cannot is already unlocked by {}",
                user_day.day.id, user.username
            ));
        }

        if user_day.day.unlocks_at > Utc::now() {
            return Err(format!(
                "day {} cannot yet be unlocked, it unlocks at {}",
                user_day.day.id, user_day.day.unlocks_at
            ));
        }

        self.get_repo().await.unlock_day(user, user_day, code).await
    }
    pub async fn get_rich_content(
        &self,
        user_day: &UserDay,
        user: &User,
    ) -> Result<String, String> {
        if user_day.unlocked_at.is_some() {
            self.get_repo().await.get_content(user_day).await
        } else {
            Err(format!(
                "day {} is not unlocked by user {}",
                user_day.day.id, user.username
            ))
        }
    }
    pub async fn delete_day(
        &self,
        user_calendar: &UserCalendar,
        user_day: &UserDay,
        user: &User,
    ) -> Result<(), String> {
        if user_calendar.calendar.owner_id == user.id {
            self.get_repo().await.delete_day(user_day).await
        } else {
            Err(format!(
                "user {} is not the owner of day {}",
                user.id, user_day.day.id
            ))
        }
    }
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: Arc::new(RwLock::new(CalendarRepository::new(pool))),
        }
    }

    async fn get_repo(&self) -> RwLockReadGuard<'_, CalendarRepository> {
        self.repo.read().await
    }

    pub async fn create_calendar(&self, user: &User, title: &str) -> Result<Calendar, String> {
        let calendar = self.get_repo().await.create_calendar(user, title).await?;
        self.repo.read().await.subscribe(user, &calendar).await?;
        Ok(calendar)
    }

    pub async fn subscribe(&self, user: &User, user_calendar: &UserCalendar) -> Result<(), String> {
        if user_calendar.subscribed_at.is_some() {
            return Err(format!(
                "user {} is already subscribed to calendar {}",
                user.username, user_calendar.calendar.title
            ));
        }

        self.repo
            .read()
            .await
            .subscribe(user, &user_calendar.calendar)
            .await
    }

    pub async fn get_dashboard_data(&self, user: &User) -> Result<Vec<RichUserCalendar>, String> {
        self.get_repo().await.get_dashboard_data(&user).await
    }

    pub async fn get_calendar_user_days(
        &self,
        user_calendar: &UserCalendar,
        user: &User,
    ) -> Result<Vec<UserDay>, String> {
        self.get_repo()
            .await
            .get_calendar_user_days(user_calendar, user)
            .await
    }

    pub async fn add_day(
        &self,
        user: &User,
        user_calendar: &UserCalendar,
        unlocks_at: DateTime<Utc>,
        password: Option<String>,
        content: String,
    ) -> Result<(), String> {
        if user.id != user_calendar.calendar.owner_id {
            return Err(format!(
                "user {} is not the owner of calendar {}",
                user.username, user_calendar.calendar.title
            ));
        }

        self.get_repo()
            .await
            .add_day(
                user,
                user_calendar,
                unlocks_at,
                password.and_then(|p| if p.is_empty() { None } else { Some(p) }),
                content,
            )
            .await
    }
}

#[derive(Deserialize)]
pub struct CalendarPath {
    pub calendar_id: i32,
}
pub async fn add_calendar(
    State(state): State<AppState>,
    Path(CalendarPath { calendar_id }): Path<CalendarPath>,
    user: User,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let cal = state
        .calendar_service
        .get_user_calendar(calendar_id, &user)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    req.extensions_mut().insert(cal);
    Ok(next.run(req).await)
}

impl<S> FromRequestParts<S> for UserCalendar
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(parts
            .extensions
            .get::<UserCalendar>()
            .cloned()
            .ok_or(StatusCode::UNAUTHORIZED)?)
    }
}

#[derive(Deserialize)]
pub struct CalendarDayPath {
    pub day_id: i32,
}
pub async fn add_calendar_day(
    State(state): State<AppState>,
    Path(CalendarDayPath { day_id }): Path<CalendarDayPath>,
    user_calendar: UserCalendar,
    user: User,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let cal = state
        .calendar_service
        .get_user_day(&user_calendar, day_id, &user)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    req.extensions_mut().insert(cal);
    Ok(next.run(req).await)
}

impl<S> FromRequestParts<S> for UserDay
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(parts
            .extensions
            .get::<UserDay>()
            .cloned()
            .ok_or(StatusCode::UNAUTHORIZED)?)
    }
}
