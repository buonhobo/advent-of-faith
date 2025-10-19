use crate::model::user::User;
use crate::web::handler::LoginCredentials;
use askama::Template;

#[derive(Template)]
#[template(path = "hello.html")]
pub struct HelloTemplate {
    user: User,
}

impl HelloTemplate {
    pub fn new(user: User) -> Self {
        HelloTemplate { user }
    }
}

struct CredentialStatusMessage {
    message: String,
    creds: LoginCredentials,
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    status_message: Option<CredentialStatusMessage>,
}
impl LoginTemplate {
    pub fn with_message(message: String, creds: LoginCredentials) -> Self {
        Self {
            status_message: Some(CredentialStatusMessage { message, creds }),
        }
    }

    pub fn empty() -> Self {
        Self {
            status_message: None,
        }
    }
}

#[derive(Template)]
#[template(path = "signup.html")]
pub struct SignupTemplate {
    status_message: Option<CredentialStatusMessage>,
}

impl SignupTemplate {
    pub fn with_message(message: String, creds: LoginCredentials) -> Self {
        Self {
            status_message: Some(CredentialStatusMessage { message, creds }),
        }
    }

    pub fn empty() -> Self {
        Self {
            status_message: None,
        }
    }
}

#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeTemplate;
