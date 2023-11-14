use std::fmt::Display;

use axum::{extract::State, Form};
use maud::Markup;
use serde::Deserialize;

use crate::{db::DB, AppState};

use super::input::{Config, Feedback, InputField};

const CONFIG: Config = Config {
    label: "Password",
    name: "password",
    kind: Some("password"),
    placeholder: None,
    validate_api: Some("./password/validate"),
    hyper_script: Some(
        "on change or keyup debounced at 350ms
            send newpass to #confirm-password
        end",
    ),
};

#[derive(Debug, PartialEq, Eq, Hash)]
enum PasswordError {
    InvalidChar(char),
    TooShort,
    TooLong,
    NoUpper,
    NoLower,
    NoNumeric,
}

impl Display for PasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PasswordError::InvalidChar(c) => write!(f, "Forbidden: {}", c),
            PasswordError::TooShort => write!(f, "Too short"),
            PasswordError::TooLong => write!(f, "Too long"),
            PasswordError::NoUpper => write!(f, "No upper-case"),
            PasswordError::NoLower => write!(f, "No lower-case"),
            PasswordError::NoNumeric => write!(f, "No number"),
        }
    }
}
#[derive(Deserialize)]
pub struct PasswordQuery {
    pub(crate) password: Box<str>,
}

const SPECIAL_CHARS: &str = " !\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";
impl PasswordQuery {
    const MIN: u8 = 6;
    const MAX: u8 = 64;

    fn validate_char(c: &char) -> bool {
        if c.is_alphabetic() {
            return true;
        }
        if c.is_numeric() {
            return true;
        }
        if SPECIAL_CHARS.contains([*c]) {
            return true;
        }
        false
    }
}

impl Feedback<PasswordError> for PasswordQuery {
    fn into_value(self) -> Box<str> {
        self.password
    }
    const CFG: &'static Config = &CONFIG;
    async fn validate(&self, _db: &DB) -> Result<(), PasswordError> {
        use PasswordError as Error;

        if let Some(c) = self.password.chars().find(|c| !Self::validate_char(c)) {
            return Err(Error::InvalidChar(c));
        };

        if (self.password.len() as u8) < Self::MIN {
            return Err(Error::TooShort);
        }
        if (self.password.len() as u8) > Self::MAX {
            return Err(Error::TooLong);
        }
        if !self.password.contains(|c: char| c.is_lowercase()) {
            return Err(Error::NoLower);
        }
        if !self.password.contains(|c: char| c.is_uppercase()) {
            return Err(Error::NoUpper);
        }
        if !self.password.contains(|c: char| c.is_numeric()) {
            return Err(Error::NoNumeric);
        }
        Ok(())
    }
}

type InputName = InputField<PasswordError>;
pub fn init_input() -> Markup {
    InputName::new(&CONFIG).into_markup()
}

pub async fn validate_handler(
    State(state): State<AppState>,
    Form(name_query): Form<PasswordQuery>,
) -> Markup {
    name_query.into_input(&state.db).await.into_markup()
}
