use std::{fmt::Display, str::FromStr};

use email_address::EmailAddress;
use maud::{html, Markup};
use serde::Deserialize;

use crate::db::DB;

#[derive(Deserialize)]
pub struct EmailQuery {
    pub email: String,
    pub id: Option<u32>,
}

impl EmailQuery {
    pub fn new(email: String) -> Self {
        Self { email, id: None }
    }
}

#[derive(Debug)]
pub enum EmailError {
    FormatError(email_address::Error),
    Occupied,
}
impl Display for EmailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmailError::FormatError(e) => write!(f, "{}", e),
            EmailError::Occupied => write!(f, "Email is occupied"),
        }
    }
}

#[derive(Debug)]
pub struct IsNew(bool);

impl Display for IsNew {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 {
            write!(f, "✅")
        } else {
            Ok(())
        }
    }
}
#[derive(Debug)]
pub struct EmailFeedBack(pub Result<(), EmailError>);
// #[derive(Debug)]
// pub struct EmailFeedBack(pub Result<IsNew, EmailError>);

impl EmailFeedBack {
    fn ok(new: bool) -> Self {
        Self(Ok(()))
    }
    fn err(e: EmailError) -> Self {
        Self(Err(e))
    }
    pub fn into_markup(self) -> Markup {
        self.into()
    }
}
impl Default for EmailFeedBack {
    fn default() -> Self {
        Self(Ok(()))
    }
}

impl From<EmailFeedBack> for Markup {
    fn from(EmailFeedBack(res): EmailFeedBack) -> Self {
        match res {
            Ok(_new) => html! { span {"✅"} },

            Err(e) => html! {
                span.alert.alert-danger.inline-err role="alert" {
                    (e)
                }
            },
        }
    }
}

/// check if email string is formated proparly and if occupied
pub async fn validate_email(q: EmailQuery, db: &DB) -> sqlx::Result<EmailFeedBack> {
    let email_res = EmailAddress::from_str(&q.email);
    if let Err(e) = email_res {
        return Ok(EmailFeedBack::err(EmailError::FormatError(e)));
    };
    match db.find_email(&q.email).await? {
        None => Ok(EmailFeedBack::ok(true)),
        Some(old_id) => match q.id {
            Some(query_id) if query_id as i32 == old_id => Ok(EmailFeedBack::ok(true)),
            _ => Ok(EmailFeedBack::err(EmailError::Occupied)),
        },
    }
}
/// check if email string is formated proparly and if occupied
pub async fn validate_user_email(
    db: &DB,
    email: &str,
    user_id: Option<u32>,
) -> sqlx::Result<EmailFeedBack> {
    let email_res = EmailAddress::from_str(email);
    if let Err(e) = email_res {
        return Ok(EmailFeedBack::err(EmailError::FormatError(e)));
    };
    match db.find_user_email(email).await? {
        None => Ok(EmailFeedBack::ok(true)),
        Some(old_id) => match user_id {
            Some(query_id) if query_id as i32 == old_id => Ok(EmailFeedBack::ok(true)),
            _ => Ok(EmailFeedBack::err(EmailError::Occupied)),
        },
    }
}
