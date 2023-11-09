use std::fmt::Display;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Router,
};

use axum_login::{
    axum_sessions::{async_session::MemoryStore, SessionLayer},
    secrecy::SecretVec,
    AuthLayer, AuthUser, MySqlStore, RequireAuthorizationLayer,
};
use log::error;
use maud::html;
use serde::Deserialize;

mod login;
mod new;
mod templates;

use crate::{
    email::{validate_user_email, EmailQuery},
    AppState,
};

use crate::templates::Markup;

use self::new::{get_create_form, post_new_user, validate_handler};

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

#[derive(Debug, Default, Clone, sqlx::FromRow)]
struct User {
    id: i32,
    password_hash: String,
    name: String,
    email: String,
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
        .route("/new", get(get_create_form))
        .route("/email/validate", get(email_validation))
        .route("/password/validate", get(validate_password_query))
        .route("/name/validate", post(validate_handler))
        .route("/logout", get(logout_handler))
        .route("/login", get(login_handler))
        .layer(auth_layer)
        .layer(session_layer)
        .with_state(app.clone())
}
