use crate::persistence::user_repository::LoginCredentials;
use askama::Template;

struct CredentialStatusMessage {
    message: String,
    creds: LoginCredentials,
}

#[derive(Template)]
#[template(path = "authentication/login.html")]
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
#[template(path = "authentication/signup.html")]
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
#[template(path = "authentication/change-password.html")]
pub struct ChangePassTemplate {
    status_message: Option<String>,
}
impl ChangePassTemplate {
    pub fn with_message(message: String) -> Self {
        Self {
            status_message: Some(message),
        }
    }

    pub fn empty() -> Self {
        Self {
            status_message: None,
        }
    }
}
