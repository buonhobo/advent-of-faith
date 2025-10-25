use crate::model::calendar::{Calendar, CalendarDay};
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
    calendar: Calendar,
    days: Vec<CalendarDay>,
    user: Option<User>,
}

impl ShowCalendarTemplate {
    pub fn new(
        calendar: Calendar,
        days: Vec<CalendarDay>,
        user: Option<User>,
    ) -> ShowCalendarTemplate {
        ShowCalendarTemplate {
            calendar,
            days,
            user,
        }
    }
}
