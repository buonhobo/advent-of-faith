use crate::domain::user::{User, UserRole};
use crate::web::handler::LoginCredentials;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use sqlx::{FromRow, PgPool};

pub struct UserRepository {
    db_pool: PgPool,
}

impl UserRepository {
    pub fn new(db_pool:PgPool) -> Self {
        UserRepository {
            db_pool
        }
    }

    pub async fn get_user(&self, user: User) -> Option<User> {
        sqlx::query_as!(
            User,
            "select id, username, role as \"role: _\" from users where username = ($1)",
            user.username
        )
        .fetch_optional(&self.db_pool)
        .await
        .unwrap()
    }

    pub async fn authenticate_user(&self, user: &LoginCredentials) -> Result<User, &str> {
        let res:UserRow = sqlx::query_as!(UserRow, "select id, username, password_hash, role as \"role: _\" from users where username = ($1)",user.username)
            .fetch_optional(&self.db_pool)
            .await
            .expect("Couldn't authenticate user")
            .ok_or("Couldn't find this user")?;

        Argon2::default()
            .verify_password(
                user.password.as_bytes(),
                &PasswordHash::new(&res.password_hash).unwrap(),
            )
            .map(|_| res.into())
            .map_err(|_| "Invalid credentials")
    }

    pub async fn add_user(&self, user: &LoginCredentials, role: UserRole) -> Result<User, String> {
        sqlx::query_as!(User, "insert into users (username,role,password_hash) values ($1,$2,$3) returning id,username,role as \"role: _\"",
            user.username,
            role as UserRole,
            Self::hash_password(&user.password)
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|_| "This username is taken".to_owned())
    }

    fn hash_password(password: &str) -> String {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        argon2
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
