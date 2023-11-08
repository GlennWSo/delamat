use std::fmt::Display;

use argon2::{
    password_hash::{
        rand_core::{OsRng, RngCore},
        SaltString,
    },
    Argon2, PasswordHasher,
};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect},
    Form,
};
use axum_flash::{Flash, IncomingFlashes, Level};
use maud::{html, Markup};
use serde::Deserialize;
use sqlx::mysql::MySqlQueryResult;

use crate::{
    db::DB,
    email::{validate_user_email, EmailError},
    templates::{dismissible_alerts, layout, MsgIterable},
    user::{
        templates::{email_input, password_input},
        PasswordQuery,
    },
    AppState,
};

mod name;
use self::name::{NameError, NameInput, NameQuery};
pub use name::validate_name;

use super::{login::create_hash, PasswordFormatError};

pub async fn get_create_form(flashes: IncomingFlashes) -> (IncomingFlashes, Markup) {
    let body = create_form_template(flashes.iter());
    (flashes, body)
}

pub fn create_form_template<T: Display>(msgs: impl MsgIterable<T>) -> Markup {
    let no_msg: Option<&str> = None;
    let no_msgs: Option<(Level, &str)> = None;
    let content = html! {
        h2 {"Create a Account"}
        form  method="post" hx-post="/user/new" hx-target="closest <body/>" "hx-target-500"="#flashes" "hx-target-406"="#flashes" {
            fieldset {
                (NameInput::Init.markup())
                div {
                    label for="email" { "email" }
                    (email_input("", "./email/validate", no_msg))
                }
                div {
                    label for="password" { "Password" }
                    (password_input("", no_msg))
                }
                div {
                    label for="confirm-password" { "Confirm Password" }
                    input #confirm-password type="password" _="
                        on newpass or change or keyup debounced at 350ms  
                        if my value equals #password.value and my value is not ''
                            remove @hidden from #repeat-ok
                            then add @hidden to #repeat-nok
                            then send confirm(ok: true) to next <button/>
                        else if my.value is not ''
                            then add @hidden to #repeat-ok
                            then remove @hidden from #repeat-nok
                            then send confirm(ok: false) to next <button/>
                        else
                            send confirm(ok: false) to next <button/>
                    "
                    ;
                    span #repeat-ok hidden {"✅"}
                    span.alert.alert-danger.inline-err hidden #repeat-nok role="alert" {
                        "Passwords do not match."
                    }
                }
                button _="
                    on load set :feedback to {password: false, email: false, confirm: false}
                        -- then add @disabled on me
                    end
                    
                    def update_me()
                        if :feedback.password and :feedback.email and :feedback.confirm
                            remove @disabled
                        else
                            log 'add @disabled'
                            
                    end
                
                    on password(ok) 
                        set :feedback.password to ok then update_me()
                    end
                    on email(ok) 
                        set :feedback.email to ok then update_me()
                    end
                    on confirm(ok) 
                        set :feedback.confirm to ok then update_me()
                    end
                    "
                    { "save" }

            }
        }
    };
    layout(content, no_msgs)
}
pub async fn post_new_user(
    State(state): State<AppState>,
    flash: Flash,
    Form(input): Form<CreateUserRequest>,
) -> impl IntoResponse {
    use NewUserError as E;
    match input.validate(&state.db).await {
        Ok(valid_input) => {
            let res = valid_input.insert(&state.db).await;
            match res {
                Ok(v) => {
                    log::info!("created new user, details: {:#?}", v);
                    (flash.success("created new user"), Redirect::to("/user/new")).into_response()
                }
                Err((input, db_error)) => {
                    let content = dismissible_alerts([(Level::Error, dbg!(db_error))]);
                    (StatusCode::INTERNAL_SERVER_ERROR, content).into_response()
                }
            }
        }
        Err((_, e)) => {
            let content = dismissible_alerts([(Level::Error, e)]);
            (StatusCode::NOT_ACCEPTABLE, content).into_response()
        }
    }
}
#[derive(Deserialize, Debug, Default)]
pub struct CreateUserRequest {
    name: String,
    password: String,
    email: String,
}
impl CreateUserRequest {
    async fn validate(self, db: &DB) -> Result<ValidNewUser, (Self, NewUserError)> {
        use NewUserError as E;
        let name = NameQuery {
            name: self.name.clone().into(),
        };
        match name.validate(db).await {
            NameInput::Invalid { error, .. } => return Err((self, E::NameError(error))),
            _ => (),
        }

        let feedback = validate_user_email(db, &self.email, None).await;
        match feedback {
            Ok(v) => match v.0 {
                Ok(_) => (),
                Err(e) => return Err((self, E::EmailError(e))),
            },
            Err(e) => return Err((self, E::DBError(e))),
        };

        let q: PasswordQuery = self.password.as_str().into();
        match q.validate() {
            Ok(_) => (),
            Err(e) => return Err((self, E::PasswordError(e))),
        };

        Ok(ValidNewUser(self))
    }
}
struct ValidNewUser(CreateUserRequest);

/// number of bytes(u8) used for storing salts
const SALT_LENGTH: usize = 16;

impl ValidNewUser {
    fn demote(self) -> CreateUserRequest {
        self.0
    }
    async fn insert(self, db: &DB) -> Result<MySqlQueryResult, (CreateUserRequest, sqlx::Error)> {
        let input = &self.0;
        let hash = create_hash(&input.password);
        let res = sqlx::query!(
            "
            insert into users (name, email, password_hash)
            values (?, ?, ?)
            ",
            input.name,
            input.email,
            hash,
        )
        .execute(db.conn())
        .await;
        match res {
            Ok(v) => Ok(v),
            Err(e) => Err((self.demote(), e)),
        }
    }
}

enum NewUserError {
    PasswordError(PasswordFormatError),
    EmailError(EmailError),
    NameError(NameError),
    DBError(sqlx::Error),
}
impl Display for NewUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NewUserError::PasswordError(e) => write!(f, "{e}"),
            NewUserError::EmailError(e) => write!(f, "{e}"),
            NewUserError::NameError(e) => write!(f, "{e}"),
            NewUserError::DBError(e) => write!(f, "{e}"),
        }
    }
}
