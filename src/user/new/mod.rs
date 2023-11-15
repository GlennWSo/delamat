use std::fmt::Display;

use axum::{
    extract::State,
    http::{StatusCode, Uri},
    response::{IntoResponse, Redirect},
    Form,
};
use axum_flash::{Flash, IncomingFlashes, Level};
use axum_htmx::responders::HxLocation;
use maud::{html, Markup};
use serde::Deserialize;
use sqlx::mysql::MySqlQueryResult;

use crate::{
    db::DB,
    email::{validate_user_email, EmailError},
    templates::{dismissible_alerts, layout, MsgIterable},
    user::{templates::password_input, PasswordQuery},
    AppState,
};

pub mod email;
mod input;
mod name;
pub mod password;

pub use input::Feedback;
pub use name::validate_handler;
use name::{NameError, NameQuery};

use super::{login::create_hash, PasswordFormatError};

pub async fn get_create_form(flashes: IncomingFlashes) -> (IncomingFlashes, Markup) {
    let body = create_form_template(flashes.iter());
    (flashes, body)
}

pub fn create_form_template<T: Display>(msgs: impl MsgIterable<T>) -> Markup {
    // let no_msg: Option<&str> = None;
    // let no_msgs: Option<(Level, &str)> = None;
    let not_firm_msg = dismissible_alerts([(Level::Error, "Passwords do not match")]);
    let form_script = format!(
        "on htmx:beforeRequest(srcElement)
                log the event then
                if #password.value is #confirm-password.value
                    log 'password confirmed'
                else
                    log srcElement.id
                    if srcElement.id is 'newUser'
                        send newPass to #confirm-password
                        then set #flashes's innerHTML to '{}'
                        then halt the event",
        not_firm_msg.into_string()
    );

    let content = html! {
        h2 {"Create a Account"}
        form #newUser
            method="post"
            novalidate
            hx-post="/user/new"
            hx-target="closest <body/>"
            hx-ext="morph"
            "hx-target-500"="#flashes"
            "hx-target-406"="#flashes"
            _=(form_script)
         {
            fieldset {
                (name::init_input())
                (email::init_input())
                (password::init_input())
                div.form-group {
                    label for="confirm-password" { "Confirm Password" }
                    input #confirm-password.form-control type="password" _="
                        on newpass or change or keyup debounced at 350ms  
                        if my value equals #password.value 
                            add @hidden to #repeat-nok then
                            remove .is-invalid from me
                            then if my value is not ''
                                -- remove @hidden from #repeat-ok
                                add .is-valid to me
                            end
                        else 
                            -- add @hidden to #repeat-ok
                            add .is-invalid to me then remove .is-valid from me
                            then remove @hidden from #repeat-nok
                    
                    "
                    ;
                    span #repeat-ok hidden {"✅"}
                    span.alert.alert-danger.inline-err hidden #repeat-nok role="alert" {
                        "Passwords do not match"
                    }
                }
                button
                {"save"}

            }
        }
    };
    layout(content, msgs)
}
pub async fn post_new_user(
    State(state): State<AppState>,
    flash: Flash,
    Form(input): Form<CreateUserRequest>,
) -> impl IntoResponse {
    match input.validate(&state.db).await {
        Ok(valid_input) => {
            let res = valid_input.insert(&state.db).await;
            match res {
                Ok(v) => {
                    log::info!("created new user, details: {:#?}", v);
                    (flash.success("created new user"), Redirect::to("/user/new")).into_response()
                }
                Err((_input, db_error)) => {
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
        if let Err(error) = name.validate(db).await {
            return Err((self, E::NameError(error)));
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
            NewUserError::PasswordError(e) => write!(f, "Password: {e}"),
            NewUserError::EmailError(e) => write!(f, "Email: {e}"),
            NewUserError::NameError(e) => write!(f, "Name: {e}"),
            NewUserError::DBError(e) => write!(f, "{e}"),
        }
    }
}
