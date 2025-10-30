use crate::model::calendar::{RichContent, RichUserCalendar};
use crate::model::user::User;
use askama::Template;

#[derive(Template)]
#[template(path = "calendar/create.html")]
pub struct CreateCalendarTemplate {
    message: Option<String>,
}

impl CreateCalendarTemplate {
    pub fn new(message: Option<String>) -> CreateCalendarTemplate {
        CreateCalendarTemplate { message }
    }
}

#[derive(Template)]
#[template(path = "calendar/show.html")]
pub struct ShowCalendarTemplate {
    calendar: RichUserCalendar,
    user: User,
}

impl ShowCalendarTemplate {
    pub fn new(calendar: RichUserCalendar, user: User) -> ShowCalendarTemplate {
        ShowCalendarTemplate { calendar, user }
    }
}

#[derive(Template)]
#[template(path = "calendar/day/show.html")]
pub struct ShowDayTemplate {
    content: RichContent,
    user: User,
}
impl ShowDayTemplate {
    pub fn new(content: RichContent, user: User) -> ShowDayTemplate {
        ShowDayTemplate { content, user }
    }
}

#[derive(Template)]
#[template(path = "calendar/day/unlock.html")]
pub struct UnlockDayTemplate {
    code: Option<String>,
    user: User,
    message: Option<String>,
    day_id: i32,
}

impl UnlockDayTemplate {
    pub fn new(code: Option<String>, user: User, day_id: i32) -> UnlockDayTemplate {
        UnlockDayTemplate {
            code,
            user,
            message: None,
            day_id,
        }
    }

    pub fn with_message(mut self, message: Option<String>) -> UnlockDayTemplate {
        self.message = message;
        self
    }
}
