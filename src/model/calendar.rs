use chacha20poly1305::aead::Aead;
use chacha20poly1305::{ChaCha20Poly1305, KeyInit};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::fmt::Display;

#[derive(Deserialize)]
pub enum Status {
    Future,
    Locked,
    Unlocked,
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Future => "future",
            Status::Locked => "locked",
            Status::Unlocked => "unlocked",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Calendar {
    pub id: i32,
    pub owner_id: i32,
    pub title: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CalendarDay {
    pub id: i32,
    pub calendar_id: i32,
    pub unlocks_at: DateTime<Utc>,
    pub protected: bool,
    // pub content: Vec<u8>,
    // pub day_key_hash: Option<String>,
    // pub content_salt: Option<Vec<u8>>,
}

impl UserDay {
    pub fn is_unlocked(&self) -> bool {
        self.unlocked_at.is_some()
    }

    pub fn is_available(&self) -> bool {
        self.day.unlocks_at < Utc::now()
    }

    pub fn get_status(&self) -> Status {
        if self.is_unlocked() {
            Status::Unlocked
        } else if self.is_available() {
            Status::Locked
        } else {
            Status::Future
        }
    }
}
#[derive(Clone)]
pub struct UserCalendar {
    pub calendar: Calendar,
    pub subscribed_at: Option<DateTime<Utc>>,
}
pub struct RichUserCalendar {
    pub calendar: UserCalendar,
    pub days: Vec<UserDay>,
}
#[derive(Debug, Clone)]
pub struct UserDay {
    pub day: CalendarDay,
    pub unlocked_at: Option<DateTime<Utc>>,
    day_key: Option<Vec<u8>>,
}

impl UserDay {
    pub fn new(
        day: CalendarDay,
        unlocked_at: Option<DateTime<Utc>>,
        day_key: Option<Vec<u8>>,
    ) -> Self {
        UserDay {
            unlocked_at,
            day_key,
            day,
        }
    }

    pub fn get_decryption_key(&self, cypher: &[u8], salt: &[u8]) -> Result<Vec<u8>, String> {
        let key = self.day_key.clone().ok_or(format!(
            "there is no decryption key for day {}",
            self.day.id
        ))?;
        let cypher = ChaCha20Poly1305::new(key.as_slice().into())
            .decrypt(salt.into(), cypher)
            .map_err(|e| format!("Decryption failed, the key is probably outdated: {:?}", e))?;
        Ok(cypher)
    }
}
