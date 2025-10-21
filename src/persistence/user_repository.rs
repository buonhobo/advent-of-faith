use crate::model::user::{User, UserRole};
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use serde::Deserialize;
use sqlx::{FromRow, PgPool};

pub struct UserRepository {
    db_pool: PgPool,
}
#[derive(Deserialize)]
pub struct LoginCredentials {
    pub username: String,
    pub password: String,
}
impl UserRepository {
    pub fn new(db_pool: PgPool) -> Self {
        UserRepository { db_pool }
    }

    pub async fn authenticate_user(&self, user: &LoginCredentials) -> Result<User, &str> {
        println!("Querying DB to authenticate user: {}", user.username);
        let res:UserRow = sqlx::query_as!(UserRow, "select id, username, password_hash, role as \"role: _\" from users where username = ($1)",user.username)
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|_|"Database connection failed")?
            .ok_or("Couldn't find this user")?;

        match Argon2::default().verify_password(
            user.password.as_bytes(),
            &PasswordHash::new(&res.password_hash)
                .map_err(|_| "Invalid password_hash from database")?,
        ) {
            Ok(_) => Ok(res.into()),
            Err(_) => Err("Invalid credentials"),
        }
    }

    pub async fn add_user(&self, user: &LoginCredentials, role: UserRole) -> Result<User, &str> {
        println!("Querying DB to add user: {}", user.username);
        sqlx::query_as!(User, "insert into users (username,role,password_hash) values ($1,$2,$3) returning id,username,role as \"role: _\"",
            user.username,
            role as UserRole,
            Self::hash_password(&user.password)
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|_| "This username is taken")
    }

    fn hash_password(password: &str) -> String {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string()
    }
}

#[derive(FromRow)]
struct UserRow {
    id: i32,
    username: String,
    password_hash: String,
    role: UserRole,
}

impl Into<User> for UserRow {
    fn into(self) -> User {
        User {
            id: self.id,
            username: self.username,
            role: self.role,
        }
    }
}
