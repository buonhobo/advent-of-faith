use crate::model::calendar::Day;
use crate::model::user::User;
use askama::Template;

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct HelloTemplate {
    user: User,
    days: Vec<Day>,
}

impl HelloTemplate {
    pub fn new(user: User, days: Vec<Day>) -> Self {
        HelloTemplate { user, days }
    }
}

#[derive(Template)]
#[template(path = "welcome.html")]
pub struct HomeTemplate {
    user: Option<User>,
}

impl HomeTemplate {
    pub fn with_user(maybe_user: Option<User>) -> Self {
        Self { user: maybe_user }
    }
}
