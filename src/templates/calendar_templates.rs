use crate::model::calendar::RichUserCalendar;
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
