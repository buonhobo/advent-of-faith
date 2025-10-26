use crate::model::app_state::Reference;
use crate::model::calendar::{Calendar, CalendarDay, RichUserCalendar, UserCalendar, UserDay};
use crate::model::user::User;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::collections::{BTreeMap, HashMap};

pub struct CalendarRepository {
    db_pool: PgPool,
}

impl CalendarRepository {
    pub fn new(pool: PgPool) -> Self {
        CalendarRepository { db_pool: pool }
    }

    pub async fn create_calendar(&self, owner: &User, title: &str) -> Result<Calendar, String> {
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

    pub async fn get_subscriptions(&self, user: &User) -> Result<Vec<UserCalendar>, String> {
        let result = sqlx::query!(
            r#"
            SELECT calendars.id, calendars.title, calendars.created_at, calendars.owner_id, subscribed_at
            FROM calendar_subscriptions
            JOIN calendars ON calendar_subscriptions.calendar_id = calendars.id
            WHERE calendar_subscriptions.user_id = $1
            "#,
            user.id
        )
            .fetch_all(&self.db_pool)
            .await
            .map_err(|e| e.to_string())?;
        let result = result
            .into_iter()
            .map(|record| UserCalendar {
                calendar: Calendar {
                    id: record.id,
                    owner_id: record.owner_id,
                    title: record.title,
                    created_at: record.created_at,
                },
                subscribed_at: record.subscribed_at,
            })
            .collect();
        Ok(result)
    }

    pub async fn subscribe(&self, user: &User, calendar: &Calendar) -> Result<(), String> {
        sqlx::query!(
            r#"
            INSERT INTO calendar_subscriptions (user_id, calendar_id)
            VALUES ($1,$2)
            "#,
            user.id,
            calendar.id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| e.to_string())
        .map(|_| ())
    }

    pub async fn get_days(&self, calendar: &Calendar) -> Result<Vec<CalendarDay>, String> {
        sqlx::query_as!(
            CalendarDay,
            r#"
            SELECT id, calendar_id, unlocks_at
            FROM calendar_days
            WHERE calendar_id = $1
            ORDER BY unlocks_at
            "#,
            calendar.id
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| e.to_string())
    }

    pub async fn get_dashboard_data(&self, user: &User) -> Result<Vec<RichUserCalendar>, String> {
        let calendars = self.get_subscriptions(user).await?;
        let calendar_ids = calendars
            .iter()
            .map(|user_calendar| user_calendar.calendar.id)
            .collect::<Vec<_>>();

        let all_days = sqlx::query!(
            r#"
            SELECT unlocked_at, unlocks_at, cd.calendar_id, cd.id as day_id
            FROM calendar_days as cd
            JOIN calendars as c ON cd.calendar_id = c.id
            LEFT JOIN user_days as ud ON cd.id = ud.day_id
            WHERE calendar_id = ANY($1)
            ORDER BY unlocks_at
            "#,
            &calendar_ids
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| e.to_string())?;

        let user_days: Vec<(i32, UserDay)> = all_days
            .into_iter()
            .map(|record| {
                (
                    record.calendar_id,
                    UserDay {
                        unlocked_at: record.unlocked_at,
                        day: CalendarDay {
                            id: record.day_id,
                            unlocks_at: record.unlocks_at,
                            calendar_id: record.calendar_id,
                        },
                    },
                )
            })
            .collect();

        let mut user_map =
            user_days
                .into_iter()
                .fold(HashMap::new(), |mut acc, (cal_id, user_day)| {
                    acc.entry(cal_id).or_insert_with(Vec::new).push(user_day);
                    acc
                });

        let calendars = calendars
            .into_iter()
            .map(|cal| RichUserCalendar {
                days: user_map.remove(&cal.calendar.id).unwrap_or(vec![]),
                calendar: cal,
            })
            .collect();

        Ok(calendars)
    }

    pub async fn add_day(&self, calendar_id: i32, unlocks_at: DateTime<Utc>) -> Result<(), String> {
        sqlx::query!(
            "INSERT INTO calendar_days (calendar_id, unlocks_at)
                VALUES ($1, $2)",
            calendar_id,
            unlocks_at
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| e.to_string())
        .map(|_| ())
    }
}
