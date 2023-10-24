use axum::{extract::State, response::IntoResponse, routing::get, Extension, Form, Router};
use axum_flash::{Flash, IncomingFlashes, Level};
use axum_login::{
    axum_sessions::{async_session::MemoryStore, SessionLayer},
    secrecy::SecretVec,
    AuthLayer, AuthUser, MySqlStore, RequireAuthorizationLayer,
};
use maud::html;
use serde::Deserialize;
// use html_macro::html;

use crate::AppState;

use crate::templates::layout;
use crate::templates::Markup;
use crate::templates::MsgIterable;

fn new_user_template<'a>(msgs: impl MsgIterable<'a>) -> Markup {
    let content = html! {
        h2 {"Create a Account"}
        form action="user/new" method="post" {
            fieldset {
                p {
                    label for="name" { "Name" }
                    input #name name="name" type="text" placeholder="your alias";
                }
                p {
                    label for="password" { "Password" }
                    input #password password="password" type="password";
                }
                button { "save" }
            }
        }
    };
    layout(content, msgs)
}

async fn new_user_handler(flashes: IncomingFlashes) -> (IncomingFlashes, Markup) {
    let body = new_user_template(flashes.iter());
    (flashes, body)
}

#[derive(Deserialize, Debug)]
struct Input {
    name: String,
    password: String,
}
async fn new_user_create(
    Form(input): Form<Input>,
    State(state): State<AppState>,
    flash: Flash,
) -> impl IntoResponse {
    let msg = (
        Level::Error,
        format!("Not yet implemtented: anyways got input:{:#?}", input).as_str(),
    );
    new_user_template(Some(msg))
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
        .route("/new", get(new_user_handler))
        .route("/logout", get(logout_handler))
        .route("/login", get(login_handler))
        .layer(auth_layer)
        .layer(session_layer)
}
