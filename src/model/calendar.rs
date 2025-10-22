use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::fmt::Display;

#[derive(Deserialize)]
pub enum Status {
    Future,
    Locked,
    Unlocked,
}
pub struct Day {
    pub number: u8,     // 1..25
    pub status: Status, // "future", "locked", or "unlocked"
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

pub struct Calendar {
    pub id: i32,
    pub owner_id: i32,
    pub title: String,
    pub created_at: DateTime<Utc>,
}

pub struct CalendarDay {
    pub id: i32,
    pub calendar_id: i32,
    pub unlocks_at: DateTime<Utc>,
    pub content: Vec<u8>,
    pub day_key_hash: Option<String>,
    pub content_salt: Option<Vec<u8>>,
}
