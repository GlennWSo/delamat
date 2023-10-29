use std::fmt::Display;

use axum::{
    extract::{Query, State},
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
use log::{error, log};
use maud::html;
use serde::Deserialize;
use sqlx::mysql::MySqlQueryResult;
// use html_macro::html;

mod templates;

use crate::{
    db::DB,
    email::{validate_email, validate_user_email, EmailError, EmailFeedBack, EmailQuery},
    AppState,
};

use self::templates::new_template;
use crate::templates::Markup;

async fn email_validation(State(state): State<AppState>, Query(q): Query<EmailQuery>) -> Markup {
    let db_res = validate_user_email(&state.db, &q.email, q.id).await;
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

const SPECIAL_CHARS: &str = " !\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";

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
    let body = new_template(flashes.iter(), None, None);
    (flashes, body)
}

#[derive(Deserialize, Debug)]
struct NewUserInput {
    name: String,
    password: String,
    email: String,
}
impl NewUserInput {
    async fn insert(self, db: &DB) -> sqlx::Result<MySqlQueryResult> {
        let salt = "badsalt";
        let hash = "badhash";
        sqlx::query!(
            "
            insert into users (password_hash, name, email, salt)
            values (?, ?, ?, ?)
            ",
            hash,
            self.name,
            self.email,
            salt
        )
        .execute(db.conn())
        .await
    }
}

enum NewUserError {
    PasswordError(PasswordFormatError),
    EmailError(EmailError),
    DBError(sqlx::Error),
}
impl Display for NewUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NewUserError::PasswordError(e) => write!(f, "{e}"),
            NewUserError::EmailError(e) => write!(f, "{e}"),
            NewUserError::DBError(e) => write!(f, "{e}"),
        }
    }
}

impl NewUserInput {
    async fn validate(&self, db: &DB) -> Result<(), NewUserError> {
        use NewUserError as E;

        let feedback = validate_user_email(db, &self.email, None).await;
        match feedback {
            Ok(v) => match v.0 {
                Ok(_) => (),
                Err(e) => return Err(E::EmailError(e)),
            },
            Err(e) => return Err(E::DBError(e)),
        };

        let q: PasswordQuery = self.password.as_str().into();
        match q.validate() {
            Ok(_) => (),
            Err(e) => return Err(E::PasswordError(e)),
        };

        Ok(())
    }
}

async fn post_new_user(
    State(state): State<AppState>,
    flash: Flash,
    Form(input): Form<NewUserInput>,
) -> impl IntoResponse {
    use NewUserError as E;
    match input.validate(&state.db).await {
        Ok(_) => {
            let res = input.insert(&state.db).await;
            match res {
                Ok(v) => {
                    log::info!("created new user, details: {:#?}", v);
                    (flash.success("created new user"), Redirect::to("/user/new")).into_response()
                }
                Err(e) => new_template([(Level::Debug, dbg!(e))], None, None).into_response(),
            }
        }
        Err(e) => match e {
            E::PasswordError(e) => new_template(None, None, Some(e)).into_response(),
            E::EmailError(e) => new_template(None, Some(e), None).into_response(),
            E::DBError(e) => new_template([(Level::Debug, dbg!(e))], None, None).into_response(),
        },
    }
}

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
        .route("/logout", get(logout_handler))
        .route("/login", get(login_handler))
        .layer(auth_layer)
        .layer(session_layer)
        .with_state(app.clone())
}
