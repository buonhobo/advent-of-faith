use crate::model::calendar::{Calendar, CalendarDay};
use crate::model::user::User;
use sqlx::PgPool;

pub struct CalendarRepository {
    db_pool: PgPool,
}

impl CalendarRepository {
    pub fn new(pool: PgPool) -> Self {
        CalendarRepository { db_pool: pool }
    }

    pub async fn create_calendar(&self, owner: User, title: String) -> Result<Calendar, String> {
        sqlx::query_as!(
            Calendar,
            r#"
            INSERT INTO calendars (owner_id, title)
            VALUES ($1, $2)
            RETURNING *
            "#,
            owner.id,
            title
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| e.to_string())
    }

    pub async fn get_calendar(&self, calendar_id: i32) -> Result<Calendar, String> {
        sqlx::query_as!(
            Calendar,
            r#"
            SELECT *
            FROM calendars
            WHERE id = $1
            "#,
            calendar_id
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| e.to_string())
    }

    pub async fn get_subscriptions(&self, user: User) -> Result<Vec<Calendar>, String> {
        sqlx::query_as!(
            Calendar,
            r#"
            SELECT calendars.id, calendars.title, calendars.created_at, calendars.owner_id
            FROM calendar_subscriptions
            JOIN calendars ON calendar_subscriptions.calendar_id = calendars.id
            WHERE calendar_subscriptions.user_id = $1
            "#,
            user.id
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| e.to_string())
    }

    pub async fn get_days(&self, calendar: Calendar) -> Result<Vec<CalendarDay>, String> {
        sqlx::query_as!(
            CalendarDay,
            r#"
            SELECT *
            FROM calendar_days
            WHERE id = $1
            "#,
            calendar.id
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| e.to_string())
    }
}
