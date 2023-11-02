use std::fmt::Display;

use argon2::{
    password_hash::{
        rand_core::{OsRng, RngCore},
        SaltString,
    },
    Argon2, PasswordHasher,
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Extension, Form, Router,
};
use axum_flash::{Flash, IncomingFlashes, Level};
use axum_login::{
    axum_sessions::{async_session::MemoryStore, SessionLayer},
    secrecy::SecretVec,
    AuthLayer, AuthUser, MySqlStore, RequireAuthorizationLayer,
};
use log::error;
use maud::html;
use serde::Deserialize;
use sqlx::mysql::MySqlQueryResult;

mod templates;

use crate::{
    db::DB,
    email::{validate_user_email, EmailError, EmailQuery},
    templates::flashy_flash,
    AppState,
};

use self::templates::{invalid_name_input, new_template, valid_name_input};
use crate::templates::Markup;

async fn email_validation(
    State(state): State<AppState>,
    Query(q): Query<EmailQuery>,
) -> impl IntoResponse {
    let db_res = validate_user_email(&state.db, &q.email, q.id).await;
    match db_res {
        Ok(email_feedback) => {
            let content: Markup = email_feedback.into();
            content.into_response()
        }
        Err(db_error) => {
            error!("{}", db_error);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                html! { span { "Internal Error" }},
            )
                .into_response()
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum PasswordFormatError {
    InvalidChar(char),
    TooShort,
    TooLong,
    NoUpper,
    NoLower,
    NoNumeric,
}

const SPECIAL_CHARS: &str = " !\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";

impl Display for PasswordFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PasswordFormatError::InvalidChar(c) => write!(f, "Forbidden: {}", c),
            PasswordFormatError::TooShort => write!(f, "Too short"),
            PasswordFormatError::TooLong => write!(f, "Too long"),
            PasswordFormatError::NoUpper => write!(f, "No upper-case"),
            PasswordFormatError::NoLower => write!(f, "No lower-case"),
            PasswordFormatError::NoNumeric => write!(f, "No number"),
        }
    }
}

#[derive(Deserialize, Debug)]
struct PasswordQuery {
    password: Box<str>,
}

impl From<&str> for PasswordQuery {
    fn from(value: &str) -> Self {
        Self {
            password: value.into(),
        }
    }
}

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

    fn validate(&self) -> Result<(), PasswordFormatError> {
        use PasswordFormatError as Error;

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
    fn get_feedback(&self) -> Markup {
        html! {
            @match self.validate(){
                Ok(_) => span #password-feedback.ok  _="
                    on load send password(ok:true) to next <button/>"
                    {"✅"},
                Err(e) => span #password-feedback.alert.alert-danger.inline-err role="alert" _="
                    on load send password(ok:false) to next <button/>"
                    {(e)},
            }
        }
    }
}

async fn validate_password_query(Query(q): Query<PasswordQuery>) -> Markup {
    q.get_feedback()
}

async fn get_new_user(flashes: IncomingFlashes) -> (IncomingFlashes, Markup) {
    let body = new_template(flashes.iter(), None, None, NewUserInput::default());
    (flashes, body)
}

#[derive(Deserialize, Debug, Default)]
struct NewUserInput {
    name: String,
    password: String,
    email: String,
}

struct ValidNewUser(NewUserInput);

impl NewUserInput {
    async fn validate(self, db: &DB) -> Result<ValidNewUser, (Self, NewUserError)> {
        use NewUserError as E;
        let name = NameInput {
            name: self.name.clone().into(),
        };
        match name.validate(db).await {
            Ok(_) => (),
            Err(e) => return Err((self, E::NameError(e.e))),
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

/// number of bytes(u8) used for storing salts
const SALT_LENGTH: usize = 16;

impl ValidNewUser {
    fn demote(self) -> NewUserInput {
        self.0
    }
    async fn insert(self, db: &DB) -> Result<MySqlQueryResult, (NewUserInput, sqlx::Error)> {
        let mut salt_bytes = [0u8; SALT_LENGTH];
        OsRng.fill_bytes(&mut salt_bytes);
        let salt = SaltString::encode_b64(&salt_bytes).expect("salt string invariant violated");
        let input = &self.0;
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(input.password.as_bytes(), &salt)
            .unwrap() // TODO remove unwrap
            .to_string();

        let res = sqlx::query!(
            "
            insert into users (name, email, salt, password_hash)
            values (?, ?, ?, ?)
            ",
            input.name,
            input.email,
            &salt_bytes[..],
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
            NewUserError::DBError(e) => write!(f, "{e}"),
            NewUserError::NameError(e) => write!(f, "{e}"),
        }
    }
}

#[derive(Deserialize)]
struct NameInput {
    name: Box<str>,
}

struct ValidName(NameInput);
enum NameError {
    TooShort,
    TooLong,
    InvalidChar(char),
    Occupied,
}
impl Display for NameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NameError::TooShort => write!(f, "name is too short"),
            NameError::TooLong => write!(f, "name is too long"),
            NameError::InvalidChar(c) => write!(f, "name has invalid char: {}", c),
            NameError::Occupied => write!(f, "name is occupied"),
        }
    }
}

struct InvalidName {
    input: NameInput,
    e: NameError,
}

impl NameInput {
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
    async fn find_user_id(&self, db: &DB) -> Option<i32> {
        sqlx::query!("select id from users where name = ?", self.name.as_ref())
            .fetch_optional(db.conn())
            .await
            .unwrap()
            .map(|r| r.id)
    }
    async fn validate(self, db: &DB) -> Result<ValidName, InvalidName> {
        use NameError as Error;

        if let Some(c) = self.name.chars().find(|c| !Self::validate_char(c)) {
            return Err(InvalidName {
                input: self,
                e: Error::InvalidChar(c),
            });
        };

        if (self.name.len() as u8) < Self::MIN {
            return Err(InvalidName {
                input: self,
                e: Error::TooShort,
            });
        }
        if (self.name.len() as u8) > Self::MAX {
            return Err(InvalidName {
                input: self,
                e: Error::TooLong,
            });
        }
        if self.find_user_id(db).await.is_some() {
            return Err(InvalidName {
                input: self,
                e: Error::Occupied,
            });
        }
        Ok(ValidName(self))
    }
    async fn into_markup(self: NameInput, db: &DB) -> Markup {
        let res = self.validate(db).await;
        match res {
            Ok(v) => valid_name_input(&v.0.name),
            Err(err) => invalid_name_input(&err.input.name, err.e),
        }
    }
}

async fn validate_name(State(state): State<AppState>, Form(input): Form<NameInput>) -> Markup {
    input.into_markup(&state.db).await
}

async fn post_new_user(
    State(state): State<AppState>,
    flash: Flash,
    Form(input): Form<NewUserInput>,
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
                    // new_template([(Level::Error, dbg!(db_error))], None, None, input)
                    //     .into_response()
                    let content = flashy_flash([(Level::Error, dbg!(db_error))]);
                    (StatusCode::INTERNAL_SERVER_ERROR, content).into_response()
                }
            }
        }
        Err((bad_input, e)) => match e {
            E::PasswordError(e) => new_template(None, None, Some(e), bad_input).into_response(),
            E::EmailError(e) => new_template(None, Some(e), None, bad_input).into_response(),
            E::DBError(e) => {
                new_template([(Level::Debug, dbg!(e))], None, None, bad_input).into_response()
            }
            _ => new_template([(Level::Debug, dbg!("unknown err"))], None, None, bad_input)
                .into_response(),
        },
    }
}

#[allow(dead_code)]
#[derive(Debug, Default, Clone, sqlx::FromRow)]
struct User {
    id: i32,
    password_hash: String,
    name: String,
    email: String,
    salt: Vec<u8>, // [u8; 16]
}

impl AuthUser<i32> for User {
    fn get_id(&self) -> i32 {
        self.id
    }
    fn get_password_hash(&self) -> axum_login::secrecy::SecretVec<u8> {
        let hash = dbg!(self.password_hash.clone());
        SecretVec::new(hash.into())
    }
}

type AuthCtx = axum_login::extractors::AuthContext<i32, User, MySqlStore<User>>;

async fn logout_handler(mut auth: AuthCtx) -> String {
    let msg = dbg!(format!("Logging out user:{:#?}", auth.current_user));
    auth.logout().await;
    msg
}

async fn login_handler(State(state): State<AppState>, mut auth: AuthCtx) -> String {
    println!("logging in");
    let user: User = sqlx::query_as!(User, "select * from users where id = 1")
        .fetch_one(state.db.conn())
        .await
        .unwrap();
    let login_res = auth.login(&user).await;
    match login_res {
        Ok(_) => dbg!("logged in".into()),
        Err(e) => dbg!(format!("failed with: {e}")),
    }
}

async fn protected_info(Extension(user): Extension<User>) -> String {
    dbg!(format!("logged in as {:#?}", user))
}

pub fn make_auth(app: &AppState) -> Router<AppState> {
    let secret = [10u8; 64];
    let session_store = MemoryStore::new();
    let session_layer = SessionLayer::new(session_store, &secret);

    let user_store = MySqlStore::<User>::new(app.db.conn().clone());
    let auth_layer = AuthLayer::new(user_store, &secret);

    Router::new()
        .route("/pro", get(protected_info))
        .route_layer(RequireAuthorizationLayer::<i32, User>::login())
        .route("/new", post(post_new_user))
        .route("/new", get(get_new_user))
        .route("/email/validate", get(email_validation))
        .route("/password/validate", get(validate_password_query))
        .route("/name/validate", post(validate_name))
        .route("/logout", get(logout_handler))
        .route("/login", get(login_handler))
        .layer(auth_layer)
        .layer(session_layer)
        .with_state(app.clone())
}
