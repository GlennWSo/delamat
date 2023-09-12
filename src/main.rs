#![feature(async_closure)]

use std::future::Future;

use askama::Template;
use axum::{
    extract::{Form, Path, State},
    response::Html,
    response::Redirect,
    routing::get,
    Router,
};
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
    let edit = EditTemplate::new(messages, c);
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

async fn post_edit(State(db): State<DB>, Form(input): Form<Input>) -> Html<String> {
    todo!()
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
        .route("/contacts/:id", get(view))
        .with_state(db);

    // build our application
    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
