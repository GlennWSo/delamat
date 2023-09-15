#![feature(async_closure)]

// use std::future::Future;

use std::str::FromStr;

use askama::Template;
use axum::{
    extract::{Form, Path, State},
    response::{Html, IntoResponse},
    response::{Redirect, Response},
    routing::{get, post},
    Router,
};
use email_address::{self, EmailAddress};
use serde::Deserialize;

use learn_htmx::{Contact, ContactsTemplate, EditTemplate, ViewTemplate, DB};

async fn view(State(db): State<DB>, Path(id): Path<u32>) -> Html<String> {
    let c = db.get_contact(id).await.expect("could not get {id}");
    let messages = ["Hi"];
    let view = ViewTemplate::with_messages(&messages, c);
    match view.render() {
        Ok(html) => html.into(),
        Err(e) => format!("failed to render ViewTemplate\n{:?}", e).into(),
    }
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

async fn home(State(db): State<DB>) -> Html<String> {
    let contacts = db.get_all_contacts().await.unwrap();
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
        .route("/contacts/:id/edit", get(get_edit))
        .route("/contacts/:id/edit", post(post_edit))
        .route("/contacts/:id", get(view))
        .with_state(db);

    // build our application
    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
