use crate::model::calendar::{
    Calendar, CalendarDay, RichContent, RichUserCalendar, UserCalendar, UserDay,
};
use crate::model::user::User;
use chacha20poly1305::aead::AeadMut;
use chacha20poly1305::{ChaCha20Poly1305, KeyInit};
use chrono::{DateTime, Utc};
use hkdf::Hkdf;
use rand::random;
use sha2::Sha256;
use sqlx::PgPool;
use std::collections::{ HashMap};

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

    pub async fn subscribe(&self, user: &User, calendar_id: i32) -> Result<(), String> {
        sqlx::query!(
            r#"
            INSERT INTO calendar_subscriptions (user_id, calendar_id)
            VALUES ($1,$2)
            "#,
            user.id,
            calendar_id
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
            SELECT id, calendar_id, unlocks_at, protected
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

    pub async fn get_user_days(
        &self,
        calendar_ids: &[i32],
        user: &User,
    ) -> Result<Vec<(i32, UserDay)>, String> {
        let all_days = sqlx::query!(
            r#"
            SELECT unlocked_at, unlocks_at, cd.calendar_id, cd.id as day_id, protected
            FROM calendar_days as cd
            JOIN calendars as c ON cd.calendar_id = c.id
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
                let cal_id = record.calendar_id;
                let user_day = UserDay {
                    unlocked_at: record.unlocked_at,
                    day: CalendarDay {
                        id: record.day_id,
                        unlocks_at: record.unlocks_at,
                        calendar_id: record.calendar_id,
                        protected: record.protected,
                    },
                };
                (cal_id, user_day)
            })
            .collect();

        Ok(user_days)
    }

    fn get_day_key(
        user: &User,
        day_key_salt: Vec<u8>,
        day_key_encr: Vec<u8>,
    ) -> Result<Vec<u8>, String> {
        let dk = ChaCha20Poly1305::new(user.content_key.as_slice().into())
            .decrypt(day_key_salt.as_slice().into(), day_key_encr.as_slice())
            .map_err(|e| e.to_string())?;

        Ok(dk)
    }

    pub async fn get_content(&self, user: &User, day_id: i32) -> Result<RichContent, String> {
        let record = sqlx::query!(
            "SELECT calendar_id, day_content.day_id, protected, day_key_salt, day_key_encr, 
                    decryption_key_salt, content_salt, content, unlocked_at,unlocks_at,owner_id,
                    title, created_at, decryption_key_encr
            FROM day_content
            join user_days on user_days.day_id = day_content.day_id
            join calendar_days on calendar_days.id = day_content.day_id
            join calendars on calendars.id = calendar_days.calendar_id
            where day_content.day_id = $1
            ",
            day_id
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or(format!(
            "The user {} has not unlocked content for day {}",
            user.username, day_id
        ))?;

        let content = if record.protected {
            let dks = record
                .day_key_salt
                .ok_or("The day is protected but there is no user day key salt")?;
            let dke = record
                .day_key_encr
                .ok_or("The day is encrypted but there is no user day key cyphertext")?;
            let decr_key_salt = record
                .decryption_key_salt
                .ok_or("The content is protected but there is no decryption key salt")?;
            let decr_key_encr = record
                .decryption_key_encr
                .ok_or("The content is protected but there is no decryption key cypher")?;
            let content_salt = record
                .content_salt
                .ok_or("The content is protected but there is no content salt")?;
            let day_key = Self::get_day_key(user, dks, dke)?;
            let decryption_key = Self::decrypt_content(&day_key, &decr_key_encr, &decr_key_salt)?;
            Self::decrypt_content(&decryption_key, &record.content, &content_salt)?
        } else {
            record.content
        };

        let content = String::from_utf8(content).map_err(|e| e.to_string())?;
        let user_day = UserDay {
            unlocked_at: record.unlocked_at,
            day: CalendarDay {
                id: record.day_id,
                calendar_id: record.calendar_id,
                unlocks_at: record.unlocks_at,
                protected: record.protected,
            },
        };
        let calendar = Calendar {
            id: record.calendar_id,
            owner_id: record.owner_id,
            title: record.title,
            created_at: record.created_at,
        };

        Ok(RichContent {
            content,
            user_day,
            calendar,
        })
    }

    fn decrypt_content(
        decryption_key: &[u8],
        cypher: &[u8],
        salt: &[u8],
    ) -> Result<Vec<u8>, String> {
        let cypher = ChaCha20Poly1305::new(decryption_key.into())
            .decrypt(salt.into(), cypher)
            .map_err(|e| format!("Decryption failed, the key is probably outdated: {:?}", e))?;
        Ok(cypher)
    }

    pub async fn get_dashboard_data(&self, user: &User) -> Result<Vec<RichUserCalendar>, String> {
        let calendars = self.get_subscriptions(user).await?;
        let calendar_ids = calendars
            .iter()
            .map(|user_calendar| user_calendar.calendar.id)
            .collect::<Vec<_>>();

        let user_days = self.get_user_days(&calendar_ids, user).await?;
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

    pub async fn get_user_calendar(
        &self,
        calendar_id: i32,
        user: &User,
    ) -> Result<RichUserCalendar, String> {
        let query = sqlx::query!(
            r#"
            SELECT calendars.id, calendars.title, calendars.created_at, calendars.owner_id, subscribed_at
            FROM calendars
            LEFT JOIN (SELECT * FROM calendar_subscriptions WHERE calendar_subscriptions.user_id = $1) as cs
            ON cs.calendar_id = calendars.id
            WHERE calendars.id = $2
            "#,
            user.id,
            calendar_id
        );

        let record = query
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("This calendar does not exist")?;

        let user_cal = UserCalendar {
            calendar: Calendar {
                id: record.id,
                owner_id: record.owner_id,
                title: record.title,
                created_at: record.created_at,
            },
            subscribed_at: record.subscribed_at,
        };
        let user_days = self
            .get_user_days([user_cal.calendar.id, user_cal.calendar.id].as_ref(), &user)
            .await?
            .into_iter()
            .map(|(_, day)| day)
            .collect();

        let rich_cal = RichUserCalendar {
            calendar: user_cal,
            days: user_days,
        };

        Ok(rich_cal)
    }

    pub async fn add_day(&self, calendar_id: i32, unlocks_at: DateTime<Utc>) -> Result<(), String> {
        sqlx::query!(
            "INSERT INTO calendar_days (calendar_id, unlocks_at, protected)
                VALUES ($1, $2, false)",
            calendar_id,
            unlocks_at
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| e.to_string())
        .map(|_| ())
    }

    pub async fn unlock_day(
        &self,
        user: &User,
        day_id: i32,
        code: Option<String>,
    ) -> Result<(), String> {
        let record = sqlx::query!(
            "SELECT decryption_key_encr, decryption_key_salt
                FROM day_content
                where day_id = $1",
            day_id
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("This day does not exist")?;

        let (dke, dks) = if let (Some(dke), Some(dks)) =
            (record.decryption_key_encr, record.decryption_key_salt)
        {
            let code = code.ok_or(String::from("A code is required for this day"))?;
            let mut day_key = [0u8; 32];
            let hk = Hkdf::<Sha256>::new(None, code.as_bytes());
            hk.expand(b"day key", &mut day_key).unwrap(); // Should never fail since lengths are always the same

            let _ = Self::decrypt_content(&day_key, &dke, &dks)?;

            let day_key_salt: [u8; 12] = random();
            let day_key_encr = ChaCha20Poly1305::new(user.content_key.as_slice().into())
                .encrypt(&day_key_salt.into(), day_key.as_slice())
                .unwrap();

            (Some(day_key_salt.to_vec()), Some(day_key_encr))
        } else {
            (None, None)
        };

        sqlx::query!(
            "INSERT INTO user_days (user_id, day_id, day_key_salt, day_key_encr) 
            VALUES ($1, $2, $3, $4)",
            user.id,
            day_id,
            dks,
            dke
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| e.to_string())
        .map(|_| ())
    }
}
