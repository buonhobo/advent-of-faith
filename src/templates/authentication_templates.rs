use crate::persistence::user_repository::LoginCredentials;
use askama::Template;
use serde::{Deserialize, Serialize};

struct CredentialStatusMessage {
    message: String,
    creds: LoginCredentials,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct NextResource {
    pub next: Option<String>,
}

impl NextResource {
    pub fn get_next_or(&self, default: &str) -> String {
        self.next.to_owned().unwrap_or(default.to_owned())
    }
}

#[derive(Template)]
#[template(path = "authentication/login.html")]
pub struct LoginTemplate {
    status_message: Option<CredentialStatusMessage>,
    next: NextResource,
}
impl LoginTemplate {
    pub fn with_message(message: String, creds: LoginCredentials) -> Self {
        Self {
            status_message: Some(CredentialStatusMessage { message, creds }),
            next: NextResource { next: None },
        }
    }

    pub fn empty() -> Self {
        Self {
            status_message: None,
            next: NextResource { next: None },
        }
    }

    pub fn with_next(mut self, next: NextResource) -> Self {
        self.next = next;
        self
    }
}

#[derive(Template)]
#[template(path = "authentication/signup.html")]
pub struct SignupTemplate {
    status_message: Option<CredentialStatusMessage>,
    next: NextResource,
}

impl SignupTemplate {
    pub fn with_message(message: String, creds: LoginCredentials) -> Self {
        Self {
            status_message: Some(CredentialStatusMessage { message, creds }),
            next: NextResource { next: None },
        }
    }

    pub fn empty() -> Self {
        Self {
            status_message: None,
            next: NextResource { next: None },
        }
    }

    pub fn with_next(mut self, next: NextResource) -> Self {
        self.next = next;
        self
    }
}
