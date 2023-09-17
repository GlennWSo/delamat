#![feature(async_closure)]

// use std::future::Future;

use askama::Template;
use axum::{
    body::StreamBody,
    extract::{Form, Path, Query, State},
    http::header,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use email_address::{self, EmailAddress};
use serde::Deserialize;
// use std::str::FromStr;

use learn_htmx::{Contact, ContactsTemplate, EditTemplate, NewTemplate, ViewTemplate, DB};

async fn view(State(db): State<DB>, Path(id): Path<u32>) -> Html<String> {
    let c = db.get_contact(id).await.expect("could not get {id}");
    let messages = ["Hi"];
    let view = ViewTemplate::with_messages(&messages, c);
    match view.render() {
        Ok(html) => html.into(),
        Err(e) => format!("failed to render ViewTemplate\n{:?}", e).into(),
    }
}

async fn get_new() -> Html<String> {
    let view = NewTemplate::new("Full Name", "name@example.org", None)
        .render()
        .unwrap();
    view.into()
}

async fn get_edit(State(db): State<DB>, Path(id): Path<u32>) -> Html<String> {
    let c = db.get_contact(id).await.expect("could not get {id}");
    let messages = vec!["Hi"];
    let edit = EditTemplate::new(messages, None, c);
    match edit.render() {
        Ok(html) => html.into(),
        Err(e) => format!("failed to render ViewTemplate\n{:?}", e).into(),
    }
}

#[derive(Deserialize, Debug)]
// #[allow(dead_code)]
struct Input {
    name: String,
    email: String,
}

async fn post_new(
    State(db): State<DB>,
    Form(input): Form<Input>,
) -> Result<Redirect, NewContactError> {
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
    let op_id = db.find_email(&input.email).await.unwrap();
    if op_id.is_some() {
        return Err(NewContactError {
            msg: "This email is already occupied".to_string(),
            ui: input,
        });
    };
    db.add_contact(input.name.to_string(), input.email.to_string())
        .await
        .unwrap();
    Ok(Redirect::to("/contacts"))
}

async fn post_edit(
    State(db): State<DB>,
    Path(id): Path<u32>,
    Form(input): Form<Input>,
) -> impl IntoResponse {
    let email_res = EmailAddress::from_str(&input.email);
    match email_res {
        Ok(_) => EditResult::Ok(id),
        Err(e) => {
            return EditResult::Error {
                id,
                msg: e.to_string(),
                ui: input,
            }
        }
    };
    let op_id = db.find_email(&input.email).await.unwrap();
    if let Some(old_id) = op_id {
        if old_id as u32 != id {
            return EditResult::Error {
                id,
                msg: "This email is already occupied".to_string(),
                ui: input,
            };
        }
    };

    if let Err(e) = db.edit_contact(id, &input.name, &input.email).await {
        panic!("{}", e);
    };

    EditResult::Ok(id)
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
    Ok(u32),
    Error { id: u32, msg: String, ui: Input },
}
impl IntoResponse for EditResult {
    fn into_response(self) -> Response {
        match self {
            EditResult::Ok(id) => {
                let re = Redirect::to(&format!("/contacts/{}", id));
                re.into_response()
            }
            EditResult::Error { id, msg, ui } => {
                let view: String = EditTemplate::new(
                    vec![],
                    Some(msg),
                    Contact {
                        id: id as i64,
                        name: ui.name,
                        email: ui.email,
                    },
                )
                .render()
                .unwrap();
                dbg!("hi");
                Html::from(view).into_response()
            }
        }
    }
}

async fn delete_contact(State(db): State<DB>, Path(id): Path<u32>) -> Redirect {
    println!("hello from delete hanlder");
    let res = db.remove_contact(id).await.unwrap();
    dbg!(res);
    Redirect::to("/contacts")
}

// #[serde_as]
#[derive(Debug, Deserialize)]
struct ContactSearch {
    // #[serde_as(as = "NoneAsEmptyString")]
    name: String,
}

async fn home(State(db): State<DB>, q: Option<Query<ContactSearch>>) -> Html<String> {
    println!("{:?}", q);
    let contacts = if let Some(q) = q {
        db.search_by_name(&q.name).await.unwrap()
    } else {
        println!("serving all contacts");
        db.get_all_contacts().await.unwrap()
    };
    let messages = ["Hi"].into();
    let view = ContactsTemplate { messages, contacts };
    match view.render() {
        Ok(html) => html.into(),
        Err(e) => format!("failed to render ViewTemplate\n{:?}", e).into(),
    }
}

async fn index() -> Redirect {
    Redirect::permanent("/contacts")
}

use futures_util::stream;
use std::{io, str::FromStr};
async fn download_archive(State(db): State<DB>) -> impl IntoResponse {
    let chunks = db
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

#[tokio::main]
async fn main() {
    let db = DB::new(5).await;

    // inject db connection into our routes
    // let home = {
    //     let db = db.clone();
    //     async move || home(db).await
    // };

    let app = Router::new()
        .route("/", get(index))
        .route("/contacts", get(home))
        .route("/contacts/download", get(download_archive))
        .route("/contacts/new", get(get_new))
        .route("/contacts/new", post(post_new))
        .route("/contacts/:id/edit", get(get_edit))
        .route("/contacts/:id/edit", post(post_edit))
        .route("/contacts/:id/delete", get(delete_contact))
        .route("/contacts/:id", get(view))
        .with_state(db);

    // build our application
    // run it with hyper on localhost:3000
    let adress = "0.0.0.0:3000";
    println!("starting server");
    axum::Server::bind(&adress.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
