use crate::model::calendar::RichUserCalendar;
use crate::model::user::User;
use askama::Template;

#[derive(Template)]
#[template(path = "home.html")]
pub struct HelloTemplate {
    user: User,
    user_calendars: Vec<RichUserCalendar>,
}

impl HelloTemplate {
    pub fn new(user: User, user_calendars: Vec<RichUserCalendar>) -> Self {
        HelloTemplate {
            user,
            user_calendars,
        }
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
