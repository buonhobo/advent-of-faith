use crate::model::calendar::{Calendar, RichContent, RichUserCalendar};
use crate::model::user::User;
use crate::persistence::calendar_repository::CalendarRepository;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard};

#[derive(Clone)]
pub struct CalendarService {
    repo: Arc<RwLock<CalendarRepository>>,
}

impl CalendarService {
    pub async fn unlock_day(
        &self,
        day_id: i32,
        user: &User,
        code: Option<String>,
    ) -> Result<(), String> {
        self.get_repo().await.unlock_day(user, day_id, code).await
    }
}

impl CalendarService {
    pub async fn get_rich_content(&self, day_id: i32, user: &User) -> Result<RichContent, String> {
        self.get_repo().await.get_content(user, day_id).await
    }
}

impl CalendarService {
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
        self.subscribe(user, calendar.id).await?;
        Ok(calendar)
    }

    pub async fn subscribe(&self, user: &User, calendar_id: i32) -> Result<(), String> {
        self.repo.read().await.subscribe(user, calendar_id).await
    }

    pub async fn get_dashboard_data(&self, user: &User) -> Result<Vec<RichUserCalendar>, String> {
        self.get_repo().await.get_dashboard_data(&user).await
    }

    pub async fn get_calendar_with_days(
        &self,
        calendar_id: i32,
        user: &User,
    ) -> Result<RichUserCalendar, String> {
        self.get_repo()
            .await
            .get_user_calendar(calendar_id, user)
            .await
    }

    pub async fn add_day(
        &self,
        user: &User,
        calendar_id: i32,
        unlocks_at: DateTime<Utc>,
    ) -> Result<(), String> {
        let calendar = self
            .get_repo()
            .await
            .get_calendar(calendar_id)
            .await
            .expect("Calendar not found");
        if user.id != calendar.owner_id {
            return Err("This user cannot edit this calendar".to_owned());
        }
        self.get_repo().await.add_day(calendar_id, unlocks_at).await
    }
}
