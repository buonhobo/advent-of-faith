use crate::model::calendar::{UserCalendar, UserDay};
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
    user_calendar: UserCalendar,
    days: Vec<UserDay>,
    user: User,
}

impl ShowCalendarTemplate {
    pub fn new(
        user_calendar: UserCalendar,
        days: Vec<UserDay>,
        user: User,
    ) -> ShowCalendarTemplate {
        ShowCalendarTemplate {
            user_calendar,
            days,
            user,
        }
    }
}

#[derive(Template)]
#[template(path = "calendar/day/show.html")]
pub struct ShowDayTemplate {
    user_day: UserDay,
    user_calendar: UserCalendar,
    content: String,
    user: User,
}
impl ShowDayTemplate {
    pub fn new(
        user_day: UserDay,
        user_calendar: UserCalendar,
        content: String,
        user: User,
    ) -> ShowDayTemplate {
        ShowDayTemplate {
            user_day,
            user_calendar,
            content,
            user,
        }
    }
}

#[derive(Template)]
#[template(path = "calendar/day/unlock.html")]
pub struct UnlockDayTemplate {
    code: Option<String>,
    day: UserDay,
    message: Option<String>,
}

impl UnlockDayTemplate {
    pub fn new(code: Option<String>, day: UserDay) -> UnlockDayTemplate {
        UnlockDayTemplate {
            code,
            day,
            message: None,
        }
    }

    pub fn with_message(mut self, message: Option<String>) -> UnlockDayTemplate {
        self.message = message;
        self
    }
}
