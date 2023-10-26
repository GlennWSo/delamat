use std::fmt::Display;

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::{get, post},
    Extension, Form, Router,
};
use axum_flash::{Flash, IncomingFlashes, Level};
use axum_login::{
    axum_sessions::{async_session::MemoryStore, SessionLayer},
    secrecy::SecretVec,
    AuthLayer, AuthUser, MySqlStore, RequireAuthorizationLayer,
};
use log::error;
use maud::{html, PreEscaped};
use serde::Deserialize;
// use html_macro::html;

use crate::{
    email::{validate_email, validate_user_email, EmailQuery},
    AppState,
};

mod templates;

use crate::templates::layout;
use crate::templates::Markup;
use crate::templates::MsgIterable;

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

fn new_user_template<T: Display>(
    msgs: impl MsgIterable<T>,
    email_feedback: Option<&str>,
) -> Markup {
    let content = html! {
        h2 {"Create a Account"}
        form action="new" method="post" {
            fieldset {
                p {
                    label for="name" { "Name" }
                    input #name name="name" type="text" placeholder="your alias";
                }
                p {
                    label for="email" { "email" }
                    (email_input("", "./email/validate", email_feedback))
                }
                p {
                    label for="password" { "Password" }
                    input #password name="password" type="password" _="
                        on change or keyup debounced at 350ms
                            send newpass to #confirm-password
                    ";
                }
                p {
                    label for="confirm-password" { "Confirm Password" }
                    input #confirm-password type="password" _="
                        on newpass or change or keyup debounced at 350ms  
                        if my value equals #password.value 
                            remove @hidden from #repeat-ok
                            add @hidden to #repeat-nok
                        else if my value is not ''
                            add @hidden to #repeat-ok
                            remove @hidden from #repeat-nok"
                    ;
                    span #repeat-ok hidden {"✅"}
                    span.alert.alert-danger hidden #repeat-nok role="alert" {
                        "passwords do not match"
                    }
                }
                button { "save" }
            }
        }
    };
    layout(content, msgs)
}

async fn get_new_user(flashes: IncomingFlashes) -> (IncomingFlashes, Markup) {
    let body = new_user_template(flashes.iter(), None);
    (flashes, body)
}

#[derive(Deserialize, Debug)]
struct NewUserInput {
    name: String,
    password: String,
    email: String,
}

#[axum::debug_handler]
async fn post_new_user(
    State(state): State<AppState>,
    // _flash: Flash,
    Form(input): Form<NewUserInput>,
) -> impl IntoResponse {
    let msg = (
        Level::Debug,
        format!("Not yet implemtented: anyways got input:{:#?}", input),
    );
    let feedback = validate_email(&state.db, EmailQuery::new(input.email));
    new_user_template(Some(msg), Some("TODO!"))
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
        .route("/logout", get(logout_handler))
        .route("/login", get(login_handler))
        .layer(auth_layer)
        .layer(session_layer)
        .with_state(app.clone())
}
