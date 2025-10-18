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
}
