use std::fmt::Display;

use axum::{extract::State, Form};
use axum_flash::Level;
use maud::{html, Markup};
use serde::Deserialize;

use crate::{db::DB, templates::inline_msg, AppState};

pub enum NameError {
    TooShort,
    TooLong,
    InvalidChar(char),
    Occupied,
    DBError(sqlx::Error),
}
impl Display for NameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NameError::TooShort => write!(f, "name is too short"),
            NameError::TooLong => write!(f, "name is too long"),
            NameError::InvalidChar(c) => write!(f, "name has invalid char: {}", c),
            NameError::Occupied => write!(f, "name is occupied"),
            NameError::DBError(e) => write!(f, "Database failed with: {}", e),
        }
    }
}
#[derive(Deserialize)]
pub struct NameQuery {
    pub(crate) name: Box<str>,
}
impl NameQuery {
    const MIN: u8 = 4;
    const MAX: u8 = 32;
    fn validate_char(c: &char) -> bool {
        if c.is_alphabetic() {
            return true;
        }
        if c.is_numeric() {
            return true;
        }
        if "_-".contains([*c]) {
            return true;
        }
        false
    }
    async fn find_user_id(&self, db: &DB) -> Result<Option<i32>, sqlx::Error> {
        Ok(
            sqlx::query!("select id from users where name = ?", self.name.as_ref())
                .fetch_optional(db.conn())
                .await?
                .map(|r| r.id),
        )
    }
    pub async fn validate(self, db: &DB) -> NameInput {
        use NameError as Error;

        if let Some(c) = self.name.chars().find(|c| !Self::validate_char(c)) {
            let e = Error::InvalidChar(c);
            return NameInput::Invalid {
                value: self.name,
                error: e,
            };
        };

        if (self.name.len() as u8) < Self::MIN {
            let e = Error::TooShort;
            return NameInput::Invalid {
                value: self.name,
                error: e,
            };
        }
        if (self.name.len() as u8) > Self::MAX {
            return NameInput::Invalid {
                value: self.name,
                error: Error::TooLong,
            };
        }
        match self.find_user_id(db).await {
            Ok(id) => {
                if id.is_some() {
                    return NameInput::Invalid {
                        value: self.name,
                        error: Error::Occupied,
                    };
                }
            }
            Err(e) => {
                return NameInput::Invalid {
                    value: self.name,
                    error: Error::DBError(e),
                }
            }
        }
        NameInput::Valid(self.name)
    }
}
pub enum NameInput {
    Init,
    Invalid { value: Box<str>, error: NameError },
    Valid(Box<str>),
}

impl Display for NameInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.markup().into_string())
    }
}
impl NameInput {
    pub fn markup(&self) -> Markup {
        html! {
            div.input_field hx-target="this" {
                label for="name" { "Name" }
                input #name
                    name="name"
                    type="text"
                    placeholder="Alias"
                    hx-post="./name/validate"
                    hx-params="*"
                    hx-trigger="change, keyup delay:350ms changed, htmx:validation:validate"
                    value=(self.value())
                    style=(self.style()) {}
                @match self {
                    NameInput::Invalid{error, ..} => (inline_msg((Level::Error, error))),
                    _ => span {},
                }


            }

        }
    }
    fn style(&self) -> &str {
        match self {
            NameInput::Init => "",
            NameInput::Invalid { .. } => "box-shadow: 0 0 3px #CC0000",
            NameInput::Valid(_) => "box-shadow: 0 0 3px #36cc00;",
        }
    }
    fn value(&self) -> &str {
        match self {
            NameInput::Init => "",
            NameInput::Invalid { value, .. } => &value,
            NameInput::Valid(value) => &value,
        }
    }
}
pub async fn validate_name(
    State(state): State<AppState>,
    Form(name_query): Form<NameQuery>,
) -> Markup {
    name_query.validate(&state.db).await.markup()
}
