use crate::model::calendar::KeyHandler;
use sqlx::{FromRow, Type};

#[derive(Clone, Debug, Type)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum UserRole {
    ADMIN,
    MEMBER,
}

#[derive(Clone, Debug, FromRow)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub role: UserRole,
    pub content_key_handler: KeyHandler,
    pub master_key_salt: Vec<u8>,
}

impl User {
    pub fn new(
        id: i32,
        username: String,
        role: UserRole,
        content_key: Vec<u8>,
        master_key_salt: Vec<u8>,
    ) -> User {
        Self {
            id,
            username,
            role,
            content_key_handler: KeyHandler::from_key(content_key),
            master_key_salt,
        }
    }
}
