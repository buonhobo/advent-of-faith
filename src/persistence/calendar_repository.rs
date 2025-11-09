use crate::model::calendar::{Calendar, CalendarDay, RichUserCalendar, UserCalendar, UserDay};
use crate::model::user::User;
use chacha20poly1305::aead::Aead;
use chacha20poly1305::{ChaCha20Poly1305, KeyInit};
use chrono::{DateTime, Utc};
use hkdf::Hkdf;
use rand::random;
use sha2::Sha256;
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
        return self
            .get_user_days_without_key(&vec![user_calendar.calendar.id], user)
            .await;
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
                let user_day = UserDay::new(day, record.unlocked_at, None);
                user_day
            })
            .collect();

        Ok(user_days)
    }

    pub async fn get_user_day_with_key(&self, day_id: i32, user: &User) -> Result<UserDay, String> {
        let record = sqlx::query!(
            r#"
            SELECT unlocked_at, unlocks_at, cd.calendar_id, cd.id as day_id, protected, day_key_salt, day_key_encr
            FROM calendar_days as cd
            LEFT JOIN (SELECT * FROM user_days WHERE user_id = $2) as ud ON cd.id = ud.day_id
            WHERE cd.id = $1
            "#,
            day_id,
            user.id
        )
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|e| e.to_string())?;

        let Some(record) = record else {
            return Err(format!("Calendar day {} not found", day_id));
        };

        let day_key = if record.protected && record.unlocked_at.is_some() {
            let key = Self::get_day_key(
                user,
                record
                    .day_key_salt
                    .ok_or(format!("Calendar day {} salt not found", day_id))?,
                record
                    .day_key_encr
                    .ok_or(format!("Calendar day {} cypher not found", day_id))?,
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
        let user_day = UserDay::new(calendar_day, record.unlocked_at, day_key);
        return Ok(user_day);
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
            .fetch_optional(&self.db_pool)
            .await;

        let Ok(Some(record)) = record else {
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
        return Ok(user_cal);
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
        .ok_or(format!("There us no content for day {}", user_day.day.id))?;

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
            let decryption_key = user_day.get_decryption_key(&decr_key_encr, &decr_key_salt)?;
            Self::decrypt_content(&decryption_key, &record.content, &content_salt)?
        } else {
            record.content
        };

        let content = String::from_utf8(content).map_err(|e| e.to_string())?;

        Ok(content)
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

    fn encrypt_content(
        decryption_key: &[u8],
        plain: &[u8],
        salt: &[u8],
    ) -> Result<Vec<u8>, String> {
        let cypher = ChaCha20Poly1305::new(decryption_key.into())
            .encrypt(salt.into(), plain)
            .map_err(|e| format!("Encryption failed: {:?}", e))?;
        Ok(cypher)
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
            protected = true;
            let day_key = Self::generate_day_key(&password);
            let (day_salt, day_cypher) = Self::encrypt_day_key(&day_key, &user.content_key);
            let (dec_key, dec_cypher, dec_salt) =
                Self::get_decryption_key_cypher_and_salt(&day_key);
            let content_salt: [u8; 12] = random();
            let content = Self::encrypt_content(&dec_key, &content.as_bytes(), &content_salt)?;

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

    fn generate_day_key(password: &str) -> [u8; 32] {
        let mut day_key = [0u8; 32];
        let hk = Hkdf::<Sha256>::new(None, password.as_bytes());
        hk.expand(b"day key", &mut day_key).unwrap(); // Should never fail since lengths are always the same
        day_key
    }

    fn encrypt_day_key(day_key: &[u8], content_key: &[u8]) -> ([u8; 12], Vec<u8>) {
        let day_key_salt: [u8; 12] = random();
        let day_key_encr = ChaCha20Poly1305::new(content_key.into())
            .encrypt(&day_key_salt.into(), day_key)
            .unwrap();
        (day_key_salt, day_key_encr)
    }

    fn get_decryption_key_cypher_and_salt(master_key: &[u8; 32]) -> ([u8; 32], Vec<u8>, [u8; 12]) {
        let content_key: [u8; 32] = random();
        let salt: [u8; 12] = random();
        let cypher = ChaCha20Poly1305::new(master_key.into())
            .encrypt(&salt.into(), content_key.as_slice())
            .unwrap();
        (content_key, cypher, salt)
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
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("This day does not exist or it can't be unlocked yet")?;

        let (dks, dke) = if let (Some(dke), Some(dks)) =
            (record.decryption_key_encr, record.decryption_key_salt)
        {
            let code = code.ok_or(String::from("A code is required for this day"))?;
            let day_key = Self::generate_day_key(&code);
            Self::decrypt_content(&day_key, &dke, &dks)?;
            let (day_key_salt, day_key_encr) = Self::encrypt_day_key(&day_key, &user.content_key);
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
}
