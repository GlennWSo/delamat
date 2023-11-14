use std::fmt::Display;

use axum::{extract::State, Form};
use maud::Markup;
use serde::Deserialize;

use crate::{db::DB, user::new::input::validate_char, AppState};

use super::input::{Config, Feedback, InputField};

const NAME_CFG: Config = Config {
    label: "Name",
    name: "name",
    kind: Some("text"),
    placeholder: Some("Alice"),
    validate_api: Some("./name/validate"),
    hyper_script: None,
};

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
    async fn find_user_id(&self, db: &DB) -> Result<Option<i32>, sqlx::Error> {
        Ok(
            sqlx::query!("select id from users where name = ?", self.name.as_ref())
                .fetch_optional(db.conn())
                .await?
                .map(|r| r.id),
        )
    }
}

impl Feedback<NameError> for NameQuery {
    fn into_value(self) -> Box<str> {
        self.name
    }
    const CFG: &'static Config = &NAME_CFG;
    async fn validate(&self, db: &DB) -> Result<(), NameError> {
        use NameError as Error;

        if let Some(c) = self.name.chars().find(|c| !validate_char(c)) {
            let e = Error::InvalidChar(c);
            return Err(e);
        };

        if (self.name.len() as u8) < Self::MIN {
            return Err(Error::TooShort);
        }
        if (self.name.len() as u8) > Self::MAX {
            return Err(Error::TooLong);
        }
        match self.find_user_id(db).await {
            Ok(id) => {
                if id.is_some() {
                    return Err(Error::Occupied);
                }
            }
            Err(e) => {
                return Err(Error::DBError(e));
            }
        }
        Ok(())
    }
}

type InputName = InputField<NameError>;
pub fn init_input() -> Markup {
    InputName::new(&NAME_CFG).into_markup()
}

pub async fn validate_handler(
    State(state): State<AppState>,
    Form(name_query): Form<NameQuery>,
) -> Markup {
    name_query.into_input(&state.db).await.into_markup()
}
