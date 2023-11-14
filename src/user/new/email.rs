use std::{fmt::Display, str::FromStr};

use axum::{extract::State, Form};
use axum_login::axum_sessions::async_session::sha2::digest::generic_array::typenum::Integer;
use email_address::EmailAddress;
use maud::Markup;
use serde::Deserialize;

use crate::{
    db::DB,
    user::new::input::{validate_char, InputState},
    AppState,
};

use super::input::{Config, Feedback, InputField};

const CFG: Config = Config {
    label: "Email",
    name: "email",
    kind: Some("text"),
    placeholder: Some("alice@example.org"),
    validate_api: Some("./email/validate"),
    hyper_script: None,
};

#[derive(Debug)]
pub enum EmailError {
    FormatError(email_address::Error),
    Occupied,
    DBError(sqlx::Error),
}
impl Display for EmailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmailError::FormatError(e) => write!(f, "{}", e),
            EmailError::Occupied => write!(f, "Email is occupied"),
            EmailError::DBError(e) => write!(f, "Database failed with: {}", e),
        }
    }
}

#[derive(Deserialize)]
pub struct EmailQuery {
    pub email: Box<str>,
    pub id: Option<u32>,
}

impl EmailQuery {
    pub fn new(email: Box<str>) -> Self {
        Self { email, id: None }
    }
}

impl Feedback<EmailError> for EmailQuery {
    fn into_value(self) -> Box<str> {
        self.email
    }
    const CFG: &'static Config = &CFG;
    async fn validate(&self, db: &DB) -> Result<(), EmailError> {
        use EmailError as Error;
        if let Err(e) = EmailAddress::from_str(&self.email) {
            return Err(Error::FormatError(e));
        };
        let id = match db.find_user_email(&self.email).await {
            Ok(id) => id,
            Err(e) => return Err(Error::DBError(e)),
        };
        match id {
            Some(_) => Err(Error::Occupied),
            None => Ok(()),
        }
    }
}

type InputEmail = InputField<EmailError>;
pub fn init_input() -> Markup {
    InputEmail::new(&CFG).into_markup()
}

pub async fn validate_handler(
    State(state): State<AppState>,
    Form(name_query): Form<EmailQuery>,
) -> Markup {
    let input = name_query.into_input(&state.db).await;
    input.into_markup()
}
