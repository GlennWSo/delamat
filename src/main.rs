#![deny(clippy::unwrap_used)]
//thirds
use axum::{
    body::StreamBody,
    extract::{Form, Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{delete, get, post},
    Router,
};
use axum_flash::{self, Flash, IncomingFlashes, Level};
use email_address::{self, EmailAddress};
use futures_util::stream;
use log::error;
use maud::{html, Markup};
use serde::Deserialize;

use terminal_link::Link;

use std::{io, str::FromStr};

use learn_htmx::{
    // auth::User,
    db::{Contact, DB},
    email::{validate_email, EmailQuery},
    templates::{self, layout},
    user::make_auth,
    AppState,
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
    let html = templates::contact::contact_details(&flashes, &c);

    Result::Ok((flashes, html))
}

async fn get_new(flashes: IncomingFlashes) -> (IncomingFlashes, Markup) {
    let content = templates::contact::new_contact("", "", None, &flashes);
    (flashes, content)
}

async fn get_edit(
    State(state): State<AppState>,
    flashes: IncomingFlashes,
    Path(id): Path<u32>,
) -> Markup {
    let c = state.db.get_contact(id).await.unwrap();
    templates::contact::edit_contact(&c, &flashes, None)
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
    let feedback = match validate_email(EmailQuery::new(input.email.clone()), &state.db).await {
        Ok(feedback) => feedback,
        Err(e) => {
            error!("db error: {}", e);
            let msg = (Level::Error, "Internal Error");
            return templates::contact::new_contact(&input.name, &input.email, None, Some(msg))
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
        Err(e) => {
            let nomsg: Option<(Level, &str)> = None;
            templates::contact::new_contact(&input.name, &input.email, Some(&e.to_string()), nomsg)
                .into_response()
        }
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
                let view: String =
                    templates::contact::edit_contact(&c, &flashes, Some(&msg)).into_string();
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
    let body = templates::contact::contact_list(&flashes, &contacts, p as u32, has_more);
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
    let db_res = validate_email(q, &state.db).await;
    match db_res {
        Ok(email_feedback) => email_feedback.into(),
        Err(db_error) => {
            error!("{}", db_error);
            html! { span { "Internal Error" }}
        }
    }
}

async fn hyper_play() -> Markup {
    let content = html! {
        div _="on load set :x to 2" {
            button _="on click <output/>"{}
            output {}

        }
    };
    let msgs: Option<(_, &str)> = None;
    layout(content, msgs)
}

#[tokio::main]
async fn main() {
    let db = DB::new(8).await;
    let app_state = AppState::new(db);
    let auth = make_auth(&app_state);

    let app = Router::new()
        .nest("/user", auth)
        .route("/", get(index))
        .route("/play", get(hyper_play))
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
