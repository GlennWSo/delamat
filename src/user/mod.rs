use std::{collections::HashSet, fmt::Display};

use axum::{
    extract::{Query, State},
    response::IntoResponse,
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
use maud::{html, PreEscaped};
use serde::Deserialize;
// use html_macro::html;

mod templates;

use crate::{
    email::{validate_email, validate_user_email, EmailQuery},
    AppState,
};

use self::templates::new_user_template;
use crate::templates::layout;
use crate::templates::Markup;
use crate::templates::MsgIterable;

async fn email_validation(State(state): State<AppState>, Query(q): Query<EmailQuery>) -> Markup {
    let db_res = validate_user_email(&state.db, q).await;
    match db_res {
        Ok(email_feedback) => email_feedback.into(),
        Err(db_error) => {
            error!("{}", db_error);
            html! { span { "Internal Error" }}
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum PasswordFormatError {
    NotAscii,
    TooShort,
    TooLong,
    NoUpper,
    NoLower,
    NoNumeric,
}

impl Display for PasswordFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PasswordFormatError::NotAscii => write!(f, "not valid ascii"),
            PasswordFormatError::TooShort => write!(f, "too short"),
            PasswordFormatError::TooLong => write!(f, "too long"),
            PasswordFormatError::NoUpper => write!(f, "no upper-case"),
            PasswordFormatError::NoLower => write!(f, "no lower-case"),
            PasswordFormatError::NoNumeric => write!(f, "no number"),
        }
    }
}

struct PasswordErrors {
    errors: HashSet<PasswordFormatError>,
}

impl Display for PasswordErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.errors.iter();
        let first = if let Some(e) = iter.next() {
            e
        } else {
            return write!(f, "✔️");
        };
        for e in iter {
            write!(f, "{e}, ")?;
        }
        write!(f, "{}", first)
    }
}

#[derive(Deserialize, Debug)]
struct PasswordQuery {
    password: Box<str>,
}

impl PasswordQuery {
    const MIN: u8 = 6;
    const MAX: u8 = 64;
    fn validate(&self) -> Result<(), PasswordErrors> {
        let mut errors: HashSet<PasswordFormatError> = HashSet::new();

        if !self.password.is_ascii() {
            errors.insert(PasswordFormatError::NotAscii);
        }

        if (self.password.len() as u8) < Self::MIN {
            errors.insert(PasswordFormatError::TooShort);
        }
        if (self.password.len() as u8) > Self::MAX {
            errors.insert(PasswordFormatError::TooLong);
        }
        if !self.password.contains(|c: char| c.is_lowercase()) {
            errors.insert(PasswordFormatError::NoLower);
        }
        if !self.password.contains(|c: char| c.is_uppercase()) {
            errors.insert(PasswordFormatError::NoUpper);
        }
        if !self.password.contains(|c: char| c.is_numeric()) {
            errors.insert(PasswordFormatError::NoNumeric);
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(PasswordErrors { errors })
        }
    }
}

async fn validate_password_api(Query(q): Query<PasswordQuery>) -> Markup {
    let feedback = q.validate();
    html! {
        @match feedback{
            Ok(_) => span {"✔️"},
            Err(e) => span.alert.alert-danger role="alert" {(e)},
        }
    }
}

async fn get_new_user(flashes: IncomingFlashes) -> (IncomingFlashes, Markup) {
    let body = new_user_template(flashes.iter(), None, None);
    (flashes, body)
}

#[derive(Deserialize, Debug)]
struct NewUserInput {
    name: String,
    password: String,
    email: String,
}

async fn post_new_user(
    State(state): State<AppState>,
    // _flash: Flash,
    Form(input): Form<NewUserInput>,
) -> impl IntoResponse {
    let msg = (
        Level::Debug,
        format!("Not yet implemtented: anyways got input:{:#?}", input),
    );
    let feedback = validate_email(EmailQuery::new(input.email), &state.db);
    new_user_template(Some(msg), Some("TODO!"), Some("TODO"))
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
        .route("/new", get(get_new_user))
        .route("/email/validate", get(email_validation))
        .route("/password/validate", get(validate_password_api))
        .route("/logout", get(logout_handler))
        .route("/login", get(login_handler))
        .layer(auth_layer)
        .layer(session_layer)
        .with_state(app.clone())
}
