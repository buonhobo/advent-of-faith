use crate::model::calendar::{
    Calendar, CalendarDay, KeyHandler, RichUserCalendar, UserCalendar, UserDay,
};
use crate::model::user::User;
use chrono::{DateTime, Utc};
use rand::random;
use sqlx::PgPool;
use std::collections::HashMap;

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

    pub async fn get_calendar_user_days(
        &self,
        user_calendar: &UserCalendar,
        user: &User,
    ) -> Result<Vec<UserDay>, String> {
        self.get_user_days_without_key(&vec![user_calendar.calendar.id], user)
            .await
    }

    pub async fn get_user_days_without_key(
        &self,
        calendar_ids: &[i32],
        user: &User,
    ) -> Result<Vec<UserDay>, String> {
        let all_days = sqlx::query!(
            r#"
            SELECT unlocked_at, unlocks_at, cd.calendar_id, cd.id as day_id, protected
            FROM calendar_days as cd
            LEFT JOIN (SELECT * FROM user_days WHERE user_id = $2) as ud ON cd.id = ud.day_id
            WHERE cd.calendar_id = ANY($1)
            ORDER BY unlocks_at
            "#,
            calendar_ids,
            user.id
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| e.to_string())?;

        let user_days = all_days
            .into_iter()
            .map(|record| {
                let day = CalendarDay {
                    id: record.day_id,
                    unlocks_at: record.unlocks_at,
                    calendar_id: record.calendar_id,
                    protected: record.protected,
                };
                let user_day = UserDay::new(day, record.unlocked_at, KeyHandler::empty());
                user_day
            })
            .collect();

        Ok(user_days)
    }

    pub async fn get_user_day_with_key(
        &self,
        user_calendar: &UserCalendar,
        day_id: i32,
        user: &User,
    ) -> Result<UserDay, String> {
        let record = sqlx::query!(
            r#"
            SELECT unlocked_at, unlocks_at, cd.calendar_id, cd.id as day_id, protected, day_key_salt, day_key_encr
            FROM calendar_days as cd
            LEFT JOIN (SELECT * FROM user_days WHERE user_id = $2) as ud ON cd.id = ud.day_id
            WHERE cd.id = $1 AND cd.calendar_id = $3
            "#,
            day_id,
            user.id,
            user_calendar.calendar.id
        )
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| format!("Calendar day {} not found: {}", day_id, e.to_string()))?;

        let day_key = if record.protected && record.unlocked_at.is_some() {
            let key = user.content_key_handler.decrypt(
                &record
                    .day_key_encr
                    .ok_or(format!("Calendar day {} cypher not found", day_id))?,
                &record
                    .day_key_salt
                    .ok_or(format!("Calendar day {} salt not found", day_id))?,
            );
            Some(key?)
        } else {
            None
        };

        let calendar_day = CalendarDay {
            id: record.day_id,
            calendar_id: record.calendar_id,
            unlocks_at: record.unlocks_at,
            protected: record.protected,
        };
        let user_day = UserDay::new(
            calendar_day,
            record.unlocked_at,
            KeyHandler::from_optional_key(day_key),
        );
        Ok(user_day)
    }

    pub async fn get_user_calendar(
        &self,
        cal_id: i32,
        user: &User,
    ) -> Result<UserCalendar, String> {
        let record = sqlx::query!(
            r#"
            SELECT subscribed_at, owner_id,created_at,title
            FROM calendars as c
            LEFT JOIN (SELECT * FROM calendar_subscriptions WHERE user_id = $2) as ud ON c.id = ud.calendar_id
            WHERE c.id = $1
            "#,
            cal_id,
            user.id
        )
            .fetch_one(&self.db_pool)
            .await;

        let Ok(record) = record else {
            return Err(format!("Calendar {} not found", cal_id));
        };

        let user_cal = UserCalendar {
            subscribed_at: record.subscribed_at,
            calendar: Calendar {
                id: cal_id,
                owner_id: record.owner_id,
                title: record.title,
                created_at: record.created_at,
            },
        };
        Ok(user_cal)
    }

    pub async fn get_content(&self, user_day: &UserDay) -> Result<String, String> {
        let record = sqlx::query!(
            "SELECT decryption_key_salt, content_salt, content, decryption_key_encr
            FROM day_content
            where day_content.day_id = $1
            ",
            user_day.day.id,
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or(format!("There is no content for day {}", user_day.day.id))?;

        let content = if user_day.day.protected {
            let decr_key_salt = record
                .decryption_key_salt
                .ok_or("The content is protected but there is no decryption key salt")?;
            let decr_key_encr = record
                .decryption_key_encr
                .ok_or("The content is protected but there is no decryption key cypher")?;
            let content_salt = record
                .content_salt
                .ok_or("The content is protected but there is no content salt")?;
            let decryption_key = user_day
                .day_key_handler
                .decrypt(&decr_key_encr, &decr_key_salt)?;
            let decryption_key = KeyHandler::from_key(decryption_key);
            decryption_key.decrypt(&record.content, &content_salt)?
        } else {
            record.content
        };

        let content = String::from_utf8(content).map_err(|e| e.to_string())?;

        Ok(content)
    }

    pub async fn get_dashboard_data(&self, user: &User) -> Result<Vec<RichUserCalendar>, String> {
        let calendars = self.get_subscriptions(user).await?;
        let calendar_ids = calendars
            .iter()
            .map(|user_calendar| user_calendar.calendar.id)
            .collect::<Vec<_>>();

        let user_days = self.get_user_days_without_key(&calendar_ids, user).await?;
        let mut user_map = user_days
            .into_iter()
            .fold(HashMap::new(), |mut acc, user_day| {
                acc.entry(user_day.day.calendar_id)
                    .or_insert_with(Vec::new)
                    .push(user_day);
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

    pub async fn add_day(
        &self,
        user: &User,
        user_calendar: &UserCalendar,
        unlocks_at: DateTime<Utc>,
        password: Option<String>,
        content: String,
    ) -> Result<(), String> {
        let tx = self.db_pool.begin().await.map_err(|e| e.to_string())?;

        let protected;
        let day_salt_opt;
        let day_cypher_opt;
        let dec_salt_opt;
        let dec_cypher_opt;
        let content_salt_opt;
        let content_bytes;
        if let Some(password) = password {
            let day_key_handler = KeyHandler::from_pass(&password, "day key");
            let day_salt: [u8; 12] = random();
            let day_cypher =
                day_key_handler.get_encrypted_key(&user.content_key_handler, &day_salt)?;
            let dec_salt: [u8; 12] = random();
            let dec_key_handler = KeyHandler::from_random(32);
            let dec_cypher = dec_key_handler.get_encrypted_key(&day_key_handler, &dec_salt)?;
            let content_salt: [u8; 12] = random();
            let content = dec_key_handler.encrypt(&content.as_bytes(), &content_salt)?;

            protected = true;
            day_salt_opt = Some(day_salt.to_vec());
            dec_salt_opt = Some(dec_salt.to_vec());
            content_salt_opt = Some(content_salt.to_vec());
            day_cypher_opt = Some(day_cypher);
            dec_cypher_opt = Some(dec_cypher);
            content_bytes = content;
        } else {
            day_salt_opt = None;
            dec_salt_opt = None;
            day_cypher_opt = None;
            dec_cypher_opt = None;
            content_salt_opt = None;
            content_bytes = content.as_bytes().to_vec();
            protected = false;
        }

        let id: i32 = sqlx::query!(
            "INSERT INTO calendar_days (calendar_id, unlocks_at, protected)
                VALUES ($1, $2, $3)
                RETURNING id",
            user_calendar.calendar.id,
            unlocks_at,
            protected
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| e.to_string())?
        .id;

        sqlx::query!(
            "INSERT INTO day_content (decryption_key_salt, decryption_key_encr, content_salt, content, day_id)
            VALUES ($1, $2, $3, $4, $5)", dec_salt_opt, dec_cypher_opt, content_salt_opt, content_bytes, id
        ).execute(&self.db_pool).await.map_err(|e| e.to_string())?;

        sqlx::query!(
            "INSERT INTO user_days (user_id, day_id, day_key_salt,day_key_encr)
            VALUES ($1, $2, $3, $4)",
            user.id,
            id,
            day_salt_opt,
            day_cypher_opt
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;

        Ok(())
    }

    pub async fn unlock_day(
        &self,
        user: &User,
        user_day: &UserDay,
        code: Option<String>,
    ) -> Result<(), String> {
        let record = sqlx::query!(
            "SELECT decryption_key_encr, decryption_key_salt
                FROM day_content
                where day_id = $1",
            user_day.day.id
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| {
            format!(
                "This day does not exist or it can't be unlocked yet: {}",
                e.to_string()
            )
        })?;

        let (dks, dke) = if let (Some(dke), Some(dks)) =
            (record.decryption_key_encr, record.decryption_key_salt)
        {
            let code = code.ok_or(String::from("A code is required for this day"))?;
            let day_key = KeyHandler::from_pass(&code, "day key");
            day_key.decrypt(&dke, &dks)?;
            let day_key_salt: [u8; 12] = random();
            let day_key_encr =
                day_key.get_encrypted_key(&user.content_key_handler, &day_key_salt)?;
            (Some(day_key_salt.to_vec()), Some(day_key_encr))
        } else {
            (None, None)
        };

        sqlx::query!(
            "INSERT INTO user_days (user_id, day_id, day_key_salt, day_key_encr) 
            VALUES ($1, $2, $3, $4)",
            user.id,
            user_day.day.id,
            dks,
            dke
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| e.to_string())
        .map(|_| ())
    }

    pub async fn delete_day(&self, user_day: &UserDay) -> Result<(), String> {
        sqlx::query!(
            "DELETE FROM public.calendar_days WHERE id = $1",
            user_day.day.id,
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| e.to_string())
        .map(|_| ())
    }

    pub async fn edit_content(&self, user_day: &UserDay, content: String) -> Result<(), String> {
        let (salt, content) = if user_day.day.protected {
            let record = sqlx::query!(
                "SELECT decryption_key_encr, decryption_key_salt
            FROM day_content
            WHERE day_id = $1",
                user_day.day.id
            )
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| {
                format!(
                    "There is no content for day {}: {}",
                    user_day.day.id,
                    e.to_string()
                )
            })?;

            let decr_key_salt = record
                .decryption_key_salt
                .ok_or("The content is protected but there is no decryption key salt")?;
            let decr_key_encr = record
                .decryption_key_encr
                .ok_or("The content is protected but there is no decryption key cypher")?;
            let decryption_key = user_day
                .day_key_handler
                .decrypt(&decr_key_encr, &decr_key_salt)?;
            let decryption_key = KeyHandler::from_key(decryption_key);
            let content_salt: [u8; 12] = random();
            let content = decryption_key.encrypt(&content.as_bytes(), &content_salt)?;

            (Some(content_salt.to_vec()), content)
        } else {
            (None, content.as_bytes().to_vec())
        };

        sqlx::query!(
            "update day_content set content = $1, content_salt = $2 where day_id = $3",
            content,
            salt,
            user_day.day.id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| e.to_string())
        .map(|_| ())
    }

    pub async fn update_password(
        &self,
        user_day: &UserDay,
        user: &User,
        password: &str,
    ) -> Result<(), String> {
        let new_day_key = KeyHandler::from_pass(&password, "day key");
        let new_day_salt: [u8; 12] = random();
        let new_day_cypher =
            new_day_key.get_encrypted_key(&user.content_key_handler, &new_day_salt)?;

        let record = sqlx::query!(
            "SELECT decryption_key_encr, decryption_key_salt
            FROM day_content
            WHERE day_id = $1",
            user_day.day.id
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| {
            format!(
                "There is no content for day {}: {}",
                user_day.day.id,
                e.to_string()
            )
        })?;

        let decr_key_salt = record
            .decryption_key_salt
            .ok_or("The content is protected but there is no decryption key salt")?;
        let decr_key_cypher = record
            .decryption_key_encr
            .ok_or("The content is protected but there is no decryption key cypher")?;
        let dec_key = user_day
            .day_key_handler
            .decrypt(&decr_key_cypher, &decr_key_salt)?;
        let dec_key = KeyHandler::from_key(dec_key);
        let decr_key_salt: [u8; 12] = random();
        let decr_key_cypher = dec_key.get_encrypted_key(&new_day_key, &decr_key_salt)?;

        let tx = self.db_pool.begin().await.map_err(|e| e.to_string())?;

        sqlx::query!(
            "update day_content 
                        set decryption_key_encr = $1, decryption_key_salt = $2 
                        where day_id = $3",
            decr_key_cypher,
            decr_key_salt.to_vec(),
            user_day.day.id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query!(
            "update user_days
                        set day_key_encr = $1, day_key_salt = $2
                        where day_id = $3 and user_id = $4",
            new_day_cypher,
            new_day_salt.to_vec(),
            user_day.day.id,
            user.id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query!(
            "delete from user_days
                        where day_id = $1 and user_id != $2",
            user_day.day.id,
            user.id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;

        Ok(())
    }

    pub async fn set_password(
        &self,
        user_day: &UserDay,
        user: &User,
        password: &str,
    ) -> Result<(), String> {
        let record = sqlx::query!(
            "SELECT content from day_content where day_id = $1",
            user_day.day.id
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| {
            format!(
                "There is no content for day {}: {}",
                user_day.day.id,
                e.to_string()
            )
        })?;
        let content = String::from_utf8(record.content).map_err(|e| e.to_string())?;
        let new_day_key = KeyHandler::from_pass(&password, "day key");
        let new_day_salt: [u8; 12] = random();
        let new_day_cypher =
            new_day_key.get_encrypted_key(&user.content_key_handler, &new_day_salt)?;
        let dec_key = KeyHandler::from_random(32);
        let decr_key_salt: [u8; 12] = random();
        let decr_key_cypher = dec_key.get_encrypted_key(&new_day_key, &decr_key_salt)?;
        let content_salt: [u8; 12] = random();
        let content_cypher = dec_key.encrypt(&content.as_bytes(), &content_salt)?;

        let tx = self.db_pool.begin().await.map_err(|e| e.to_string())?;

        sqlx::query!(
            "update day_content 
                        set decryption_key_encr = $1, decryption_key_salt = $2, content = $3, content_salt = $4 
                        where day_id = $5",
            decr_key_cypher,
            decr_key_salt.to_vec(),
            content_cypher,
            content_salt.to_vec(),
            user_day.day.id
        )
            .execute(&self.db_pool)
            .await
            .map_err(|e| e.to_string())?;

        sqlx::query!(
            "update user_days 
                        set day_key_encr = $1, day_key_salt = $2 
                        where day_id = $3 and user_id = $4",
            new_day_cypher,
            new_day_salt.to_vec(),
            user_day.day.id,
            user.id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query!(
            "delete from user_days
                        where day_id = $1 and user_id != $2",
            user_day.day.id,
            user.id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query!(
            "update calendar_days 
                        set protected = true
                        where id = $1",
            user_day.day.id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;

        Ok(())
    }
    pub async fn remove_password(&self, user_day: &UserDay) -> Result<(), String> {
        let record = sqlx::query!(
            "SELECT decryption_key_encr, decryption_key_salt, content_salt, content
            FROM day_content
            WHERE day_id = $1",
            user_day.day.id
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| {
            format!(
                "There is no content for day {}: {}",
                user_day.day.id,
                e.to_string()
            )
        })?;

        let decr_key_salt = record
            .decryption_key_salt
            .ok_or("The content is protected but there is no decryption key salt")?;
        let content_salt = record
            .content_salt
            .ok_or("The content is protected but there is no content salt")?;
        let decr_key_cypher = record
            .decryption_key_encr
            .ok_or("The content is protected but there is no decryption key cypher")?;
        let dec_key = user_day
            .day_key_handler
            .decrypt(&decr_key_cypher, &decr_key_salt)?;
        let dec_key = KeyHandler::from_key(dec_key);
        let content = dec_key.decrypt(&record.content, &content_salt)?;

        let tx = self.db_pool.begin().await.map_err(|e| e.to_string())?;

        sqlx::query!(
            "update day_content
                        set decryption_key_encr = null, decryption_key_salt = null, content = $1, content_salt = null
                        where day_id = $2",
            content,
            user_day.day.id
        )
            .execute(&self.db_pool)
            .await
            .map_err(|e| e.to_string())?;

        sqlx::query!(
            "update user_days
                        set day_key_encr = null, day_key_salt = null
                        where day_id = $1 ",
            user_day.day.id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query!(
            "update calendar_days
                        set protected = false
                        where id = $1",
            user_day.day.id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;

        Ok(())
    }
}
