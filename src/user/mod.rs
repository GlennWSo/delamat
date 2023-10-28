use std::fmt::Display;

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::{get, post},
    Extension, Form, Router,
};
use axum_flash::{IncomingFlashes, Level};
use axum_login::{
    axum_sessions::{async_session::MemoryStore, SessionLayer},
    secrecy::SecretVec,
    AuthLayer, AuthUser, MySqlStore, RequireAuthorizationLayer,
};
use log::error;
use maud::html;
use serde::Deserialize;
// use html_macro::html;

mod templates;

use crate::{
    email::{validate_email, validate_user_email, EmailQuery},
    AppState,
};

use self::templates::new_user_template;
use crate::templates::Markup;

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
    NotAscii(char),
    TooShort,
    TooLong,
    NoUpper,
    NoLower,
    NoNumeric,
}

// const SPECIAL_CHARS: &str = " !\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";

impl Display for PasswordFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PasswordFormatError::NotAscii(c) => write!(f, "Forbidden: {}", c),
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

impl PasswordQuery {
    const MIN: u8 = 6;
    const MAX: u8 = 64;

    fn validate(&self) -> Result<(), PasswordFormatError> {
        use PasswordFormatError as Error;

        if let Some(c) = self.password.chars().find(|c| !c.is_ascii()) {
            return Err(Error::NotAscii(c));
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
                Ok(_) => span {"✔️"},
                Err(e) => span.alert.alert-danger role="alert" {(e)},
            }
        }
    }
}

async fn validate_password_query(Query(q): Query<PasswordQuery>) -> Markup {
    q.get_feedback()
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
        .route("/password/validate", get(validate_password_query))
        .route("/logout", get(logout_handler))
        .route("/login", get(login_handler))
        .layer(auth_layer)
        .layer(session_layer)
        .with_state(app.clone())
}
