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

#[derive(Debug)]
pub struct Calendar {
    pub id: i32,
    pub owner_id: i32,
    pub title: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct CalendarDay {
    pub id: i32,
    pub calendar_id: i32,
    pub unlocks_at: DateTime<Utc>,
    // pub content: Vec<u8>,
    // pub day_key_hash: Option<String>,
    // pub content_salt: Option<Vec<u8>>,
}

impl UserDay {
    fn is_unlocked(&self) -> bool {
        self.unlocked_at.is_some()
    }

    fn is_available(&self) -> bool {
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

pub struct RichCalendar {
    pub calendar: Calendar,
    pub days: Vec<CalendarDay>,
}

pub struct UserCalendar {
    pub calendar: Calendar,
    pub subscribed_at: DateTime<Utc>,
}
pub struct RichUserCalendar {
    pub calendar: UserCalendar,
    pub days: Vec<UserDay>,
}
#[derive(Debug)]
pub struct UserDay {
    pub day: CalendarDay,
    pub unlocked_at: Option<DateTime<Utc>>,
}
