use chacha20poly1305::aead::Aead;
use chacha20poly1305::{ChaCha20Poly1305, KeyInit};
use chrono::{DateTime, Utc};
use hkdf::Hkdf;
use rand::random;
use serde::Deserialize;
use sha2::Sha256;
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
    pub day_key_handler: KeyHandler,
}

#[derive(Debug, Clone)]
pub struct KeyHandler {
    key: Option<Vec<u8>>,
}

impl KeyHandler {
    pub fn empty() -> Self {
        KeyHandler { key: None }
    }
    pub fn from_key(key: Vec<u8>) -> KeyHandler {
        KeyHandler { key: Some(key) }
    }

    pub fn from_random(size: usize) -> KeyHandler {
        let key = (0..size).map(|_| random()).collect();
        KeyHandler { key: Some(key) }
    }

    pub fn from_optional_key(key: Option<Vec<u8>>) -> KeyHandler {
        KeyHandler { key }
    }

    pub fn from_pass(password: &str, context: &str) -> KeyHandler {
        KeyHandler {
            key: Some(Self::get_key_from_string(password, context)),
        }
    }

    fn get_key_from_string(string: &str, context: &str) -> Vec<u8> {
        let mut key = [0u8; 32];
        let hk = Hkdf::<Sha256>::new(None, string.as_bytes());
        hk.expand(context.as_bytes(), &mut key).unwrap(); // Should never fail since lengths are always the same
        key.to_vec()
    }

    fn get_key(&self) -> Result<Vec<u8>, String> {
        self.key.clone().ok_or(String::from("key empty"))
    }

    pub fn decrypt(&self, cypher: &[u8], salt: &[u8]) -> Result<Vec<u8>, String> {
        let cypher = ChaCha20Poly1305::new(self.get_key()?.as_slice().into())
            .decrypt(salt.into(), cypher)
            .map_err(|e| format!("Decryption failed, the key is probably outdated: {:?}", e))?;
        Ok(cypher)
    }

    pub fn encrypt(&self, secret: &[u8], salt: &[u8]) -> Result<Vec<u8>, String> {
        let cypher = ChaCha20Poly1305::new(self.get_key()?.as_slice().into())
            .encrypt(salt.into(), secret)
            .unwrap();
        Ok(cypher)
    }

    pub fn get_encrypted_key(&self, encryption_key: &Self, salt: &[u8]) -> Result<Vec<u8>, String> {
        encryption_key.encrypt(&self.get_key()?, salt)
    }
}

impl UserDay {
    pub fn new(
        day: CalendarDay,
        unlocked_at: Option<DateTime<Utc>>,
        key_handler: KeyHandler,
    ) -> Self {
        UserDay {
            unlocked_at,
            day_key_handler: key_handler,
            day,
        }
    }
}
