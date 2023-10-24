use axum::{extract::State, response::IntoResponse, routing::get, Extension, Router};
use axum_flash::IncomingFlashes;
use axum_login::{
    axum_sessions::{async_session::MemoryStore, SessionLayer},
    secrecy::SecretVec,
    AuthLayer, AuthUser, MySqlStore, RequireAuthorizationLayer,
};
use maud::html;
// use html_macro::html;

use crate::{templates::contact::new_contact, AppState};

use crate::templates::layout;
use crate::templates::Markup;
use crate::templates::MsgIterable;

fn make_user_template<'a>(msgs: impl MsgIterable<'a>) -> Markup {
    let content = html! {
        h2 {"Create a Account"}
        form action="user/new" method="post" {
            fieldset {
                p {
                    label for="name" { "Name" }
                    input #name name="name" type="text" placeholder="your alias";
                }
                p {
                    label for="email" { "Email" }
                    input #email email="email" type="email" placeholder="you@example.org";
                }
                button { " save" }
            }
        }
    };
    layout(content, msgs)
}

async fn new_user_handler(
    State(state): State<AppState>,
    flashes: IncomingFlashes,
) -> (IncomingFlashes, Markup) {
    let body = make_user_template(flashes.iter());
    (flashes, body)
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
