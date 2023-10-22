#![deny(clippy::unwrap_used)]
//thirds
use axum::{
    body::StreamBody,
    extract::{Form, FromRef, Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{delete, get, post},
    Extension, Router,
};
use axum_flash::{self, Flash, IncomingFlashes, Key, Level};
use email_address::{self, EmailAddress};
use futures_util::stream;
use log::error;
use maud::{html, Markup};
use serde::Deserialize;
use sqlx::{mysql::MySqlPoolOptions, MySqlPool};
use terminal_link::Link;

use axum_login::{
    axum_sessions::{async_session::MemoryStore, SessionLayer},
    secrecy::SecretVec,
    AuthLayer, AuthUser, MySqlStore, RequireAuthorizationLayer,
};

use std::{io, str::FromStr};

use learn_htmx::{
    db::{Contact, DB},
    email::{validate_email, EmailFeedBack, EmailQuery},
    templates,
};

async fn view(
    State(state): State<AppState>,
    flashes: IncomingFlashes,
    Path(id): Path<u32>,
) -> impl IntoResponse {
    let c = match state.db.get_contact(id).await {
        Ok(c) => c,
        Err(e) => match e {
            sqlx::Error::RowNotFound => {
                return Err((
                    (StatusCode::NOT_FOUND),
                    format!("Error: contact {} was not found", id),
                ))
            }
            e => {
                dbg!(&e, e.to_string());
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "The server failed".to_string(),
                ));
            }
        },
    };
    let html = templates::contact_details(&flashes, &c);

    Result::Ok((flashes, html))
}

async fn get_new(flashes: IncomingFlashes) -> (IncomingFlashes, Markup) {
    let content = templates::new_contact("", "", None, &flashes);
    (flashes, content)
}

async fn get_edit(
    State(state): State<AppState>,
    flashes: IncomingFlashes,
    Path(id): Path<u32>,
) -> Markup {
    let c = state.db.get_contact(id).await.unwrap();
    templates::edit_contact(&c, &flashes, None)
}

#[derive(Deserialize, Debug)]
// #[allow(dead_code)]
struct Input {
    name: String,
    email: String,
}

async fn post_new(
    State(state): State<AppState>,
    flash: Flash,
    Form(input): Form<Input>,
) -> impl IntoResponse {
    let feedback = match validate_email(&state.db, EmailQuery::new(input.email.clone())).await {
        Ok(feedback) => feedback,
        Err(e) => {
            error!("db error: {}", e);
            let msg = (Level::Error, "Internal Error".into());
            return templates::new_contact(&input.name, &input.email, None, Some(msg))
                .into_response();
        }
    };

    match feedback.0 {
        Ok(_v) => {
            state.db.add_contact(input.name, input.email).await.unwrap();
            (
                flash.success("Added new contact!"),
                Redirect::to("/contacts"),
            )
                .into_response()
        }
        Err(e) => templates::new_contact(&input.name, &input.email, Some(&e.to_string()), None)
            .into_response(),
    }
}

async fn post_edit(
    State(state): State<AppState>,
    flash: Flash,
    flashes: IncomingFlashes,
    Path(id): Path<u32>,
    Form(input): Form<Input>,
) -> EditResult {
    let email_res = EmailAddress::from_str(&input.email);
    if let Err(e) = email_res {
        {
            return EditResult::Error {
                id,
                msg: e.to_string().into(),
                ui: input,
                flashes,
            };
        }
    };
    let op_id = state.db.find_email(&input.email).await.unwrap();
    if let Some(old_id) = op_id {
        if old_id as u32 != id {
            return EditResult::Error {
                id,
                msg: "This email is already occupied".into(),
                ui: input,
                flashes,
            };
        }
    };

    if let Err(e) = state.db.edit_contact(id, &input.name, &input.email).await {
        panic!("{}", e);
    };

    EditResult::Ok(id, flash.success("Changed Saved"))
}

enum EditResult {
    Ok(u32, Flash),
    Error {
        id: u32,
        msg: Box<str>,
        ui: Input,
        flashes: IncomingFlashes,
    },
}
impl IntoResponse for EditResult {
    fn into_response(self) -> Response {
        match self {
            EditResult::Ok(id, flash) => {
                let re = Redirect::to(&format!("/contacts/{}", id));
                (flash, re).into_response()
            }
            EditResult::Error {
                id,
                msg,
                ui,
                flashes,
            } => {
                let c = Contact {
                    id: id as i32,
                    name: ui.name,
                    email: ui.email,
                };
                let view: String = templates::edit_contact(&c, &flashes, Some(&msg)).into_string();
                Html::from(view).into_response()
            }
        }
    }
}

async fn delete_contact(
    State(state): State<AppState>,
    Path(id): Path<u32>,
    flash: Flash,
) -> (Flash, Redirect) {
    state.db.remove_contact(id).await.unwrap();
    (flash.success("Hi"), Redirect::to("/contacts"))
}

// #[serde_as]
#[derive(Debug, Deserialize)]
struct ContactSearch {
    // #[serde_as(as = "NoneAsEmptyString")]
    name: String,
}

#[derive(Debug, Deserialize)]
struct Page {
    page: u32,
}

async fn set_flash(flash: Flash) -> (Flash, Redirect) {
    (flash.debug("Hi from flas!"), Redirect::to("/get_flash"))
}
async fn get_flash(flashes: IncomingFlashes) -> String {
    flashes
        .into_iter()
        .map(|m| format!("lvl:{:?} msg:{}", m.0, m.1))
        .collect()
}

async fn home(
    State(state): State<AppState>,
    flashes: IncomingFlashes,
    q: Option<Query<ContactSearch>>,
    p: Option<Query<Page>>,
) -> (IncomingFlashes, Markup) {
    let p = p.map_or(1, |p| p.0.page) as usize;
    let page_size = 10;
    let skiped = (p - 1) * page_size;
    let skiped = skiped as i64;

    let mut contacts = if let Some(q) = q {
        state.db.search_by_name(&q.name).await.unwrap()
    } else {
        println!("serving all contacts");
        let conn = state.db.conn();
        let sql_res = sqlx::query_as!(
            Contact,
            "select * from contacts
            limit 11 offset ?",
            skiped
        )
        .fetch_all(conn)
        .await;
        sql_res.unwrap_or_else(|e| {
            error!("{e}");
            vec![]
        })
    };

    // let has_more = contacts.len() > (p * page_size);
    dbg!(&contacts.len());
    let has_more = contacts.len() > 10;
    contacts.truncate(10);
    dbg!(&flashes);
    let body = templates::contact_list(&flashes, &contacts, p as u32, has_more);
    (flashes, body)
}

async fn index() -> Redirect {
    Redirect::permanent("/contacts")
}

async fn download_archive(State(state): State<AppState>) -> impl IntoResponse {
    let chunks = state
        .db
        .get_all_contacts()
        .await
        .unwrap()
        .into_iter()
        .map(|c| io::Result::Ok(format!("name: '{}'\temail: '{}'\n", c.name, c.email)));
    let stream = stream::iter(chunks);

    let headers = [
        (header::CONTENT_TYPE, "text/toml; charset=utf-8"),
        (
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"contacts.txt\"",
        ),
    ];
    (headers, StreamBody::new(stream))
}

async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "nothing to see here")
}

async fn email_validation(State(state): State<AppState>, Query(q): Query<EmailQuery>) -> Markup {
    let db_res = validate_email(&state.db, q).await;
    match db_res {
        Ok(email_feedback) => email_feedback.into(),
        Err(db_error) => {
            error!("{}", db_error);
            html! { span { "Internal Error" }}
        }
    }
}

#[derive(Clone)]
struct AppState {
    db: DB,
    flash_config: axum_flash::Config,
}
impl FromRef<AppState> for axum_flash::Config {
    fn from_ref(state: &AppState) -> Self {
        state.flash_config.clone()
    }
}

#[derive(Debug, Default, Clone, sqlx::FromRow)]
struct User {
    id: i32,
    password_hash: String,
    name: String,
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

const DB_URL: &'static str = env!("DATABASE_URL");

#[tokio::main]
async fn main() {
    let db = DB::new(5).await;
    let app_state = AppState {
        db,
        // The key should probably come from configuration
        flash_config: axum_flash::Config::new(Key::generate()),
    };

    let secret = [10u8; 64];

    let session_store = MemoryStore::new();
    let session_layer = SessionLayer::new(session_store, &secret);

    let pool = MySqlPoolOptions::new()
        .max_connections(2)
        .connect(DB_URL)
        .await
        .unwrap();
    let user_store = MySqlStore::<User>::new(pool);
    let auth_layer = AuthLayer::new(user_store, &secret);

    async fn logout_handler(mut auth: AuthCtx) -> String {
        let msg = dbg!(format!("Logging out user:{:#?}", auth.current_user));
        auth.logout().await;
        msg
    }
    async fn login_handler(mut auth: AuthCtx) -> String {
        println!("logging in");
        let pool = MySqlPoolOptions::new()
            .max_connections(2)
            .connect(DB_URL)
            .await
            .unwrap();
        let mut conn = pool.acquire().await.unwrap();
        let user: User = sqlx::query_as!(User, "select * from users where id = 1")
            .fetch_one(&mut conn)
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

    async fn derp() -> String {
        "derp".to_string()
    }

    let app = Router::new()
        .route("/pro", get(protected_info))
        .route_layer(RequireAuthorizationLayer::<i32, User>::login())
        .route("/logout", get(logout_handler))
        .route("/login", get(login_handler))
        .layer(auth_layer)
        .layer(session_layer)
        .route("/", get(index))
        .route("/contacts", get(home))
        .route("/contacts/download", get(download_archive))
        .route("/contacts/new", get(get_new))
        .route("/contacts/new", post(post_new))
        .route("/contacts/:id/edit", get(get_edit))
        .route("/contacts/:id/edit", post(post_edit))
        .route("/contacts/email", get(email_validation))
        .route("/contacts/:id", delete(delete_contact))
        .route("/contacts/:id", get(view))
        .route("/set_flash", get(set_flash))
        .route("/get_flash", get(get_flash))
        .fallback(handler_404)
        .with_state(app_state);

    // build our application
    // run it with hyper on localhost:3000
    let port = 1111;
    let adress = format!("0.0.0.0:{port}");
    let url = format!("http://127.0.0.1:{port}");
    let link = Link::new(&url, &url);
    println!("starting server {}", link);
    axum::Server::bind(&adress.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
