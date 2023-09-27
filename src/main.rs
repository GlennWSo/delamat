

use askama::Template;
use axum::{
    body::StreamBody,
    extract::{Form, FromRef, Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{delete, get, post},
    Router,
};
use axum_flash::{self, Flash, IncomingFlashes, Key};
use email_address::{self, EmailAddress};
use serde::Deserialize;

use learn_htmx::templates::{contact_list, edit_contact, NewTemplate};
use learn_htmx::{
    db::{Contact, DB},
    templates::contact_details,
};

use maud::Markup;

async fn view(
    State(state): State<AppState>,
    flashes: IncomingFlashes,
    Path(id): Path<u32>,
) -> impl IntoResponse
// where
    // T: IntoResponse,
{
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
    // let messages: Box<_> = flashes.iter().map(|(_, txt)| txt).collect();
    let html = contact_details(&flashes, &c);

    Result::Ok((flashes, html))
}

async fn get_new() -> Html<String> {
    let view = NewTemplate::new("", "", None).render().unwrap();
    view.into()
}

async fn get_edit(
    State(state): State<AppState>,
    flashes: IncomingFlashes,
    Path(id): Path<u32>,
) -> Markup {
    let c = state.db.get_contact(id).await.expect("could not get {id}");
    edit_contact(&c, &flashes, None)
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
) -> Result<(Flash, Redirect), NewContactError> {
    let email_res = EmailAddress::from_str(&input.email);
    match email_res {
        Ok(_) => (),
        Err(e) => {
            return Err(NewContactError {
                msg: e.to_string(),
                ui: input,
            })
        }
    };
    let op_id = state.db.find_email(&input.email).await.unwrap();
    if op_id.is_some() {
        return Err(NewContactError {
            msg: "This email is already occupied".to_string(),
            ui: input,
        });
    };
    state
        .db
        .add_contact(input.name.to_string(), input.email.to_string())
        .await
        .unwrap();
    Ok((flash.debug("New Contact Saved"), Redirect::to("/contacts")))
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

struct NewContactError {
    msg: String,
    ui: Input,
}
impl IntoResponse for NewContactError {
    fn into_response(self) -> Response {
        let view = NewTemplate::new(&self.ui.name, &self.ui.email, Some(self.msg))
            .render()
            .unwrap();
        let html = Html::from(view);
        html.into_response()
    }
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
                    id: id as i64,
                    name: ui.name,
                    email: ui.email,
                };
                let view: String = edit_contact(&c, &flashes, Some(&msg)).into_string();
                Html::from(view).into_response()
            }
        }
    }
}

async fn delete_contact(State(state): State<AppState>, Path(id): Path<u32>) -> Redirect {
    state.db.remove_contact(id).await.unwrap();
    Redirect::to("/contacts")
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

async fn home(
    State(state): State<AppState>,
    flashes: IncomingFlashes,
    q: Option<Query<ContactSearch>>,
    p: Option<Query<Page>>,
) -> (IncomingFlashes, Markup) {
    println!("{:?}", q);
    let contacts = if let Some(q) = q {
        state.db.search_by_name(&q.name).await.unwrap()
    } else {
        println!("serving all contacts");
        state.db.get_all_contacts().await.unwrap()
    };

    let p = p.map_or(1, |p| p.0.page) as usize;
    let page_size = 10;
    let skiped = (p - 1) * page_size;
    let has_more = contacts.len() > dbg!(p * page_size);
    let contacts: Box<[Contact]> = contacts.into_iter().skip(skiped).take(10).collect();

    let body = contact_list(&flashes, &contacts, p as u32, has_more);
    (flashes, body)
}

async fn index() -> Redirect {
    Redirect::permanent("/contacts")
}

use futures_util::stream;
use std::{io, str::FromStr};
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

#[derive(Deserialize)]
struct EmailQuery {
    email: String,
    id: Option<u32>,
}
async fn email_validation(
    State(state): State<AppState>,
    Query(q): Query<EmailQuery>,
) -> Html<String> {
    let email_res = EmailAddress::from_str(&q.email);
    if let Err(e) = email_res {
        return Html(format!(
            "<span class='alert alert-danger' role='alert'>{}</span>",
            e
        ));
    }
    match state.db.find_email(&q.email).await.unwrap() {
        Some(old_id) => {
            if let Some(qid) = q.id {
                if qid != old_id as u32 {
                    Html(format!(
                        "<span class='alert alert-danger' role='alert'>{}</span>",
                        "Occupied"
                    ))
                } else {
                    Html("<span></span>".to_string())
                }
            } else {
                Html(format!(
                    "<span class='alert alert-danger' role='alert'>{}</span>",
                    "Occupied"
                ))
            }
        }
        None => Html("<span>✅</span>".to_string()),
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

#[tokio::main]
async fn main() {
    let db = DB::new(5).await;
    let app_state = AppState {
        db,
        // The key should probably come from configuration
        flash_config: axum_flash::Config::new(Key::generate()),
    };

    let app = Router::new()
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
        .fallback(handler_404)
        .with_state(app_state);

    // build our application
    // run it with hyper on localhost:3000
    let adress = "0.0.0.0:3000";
    println!("starting server");
    axum::Server::bind(&adress.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
