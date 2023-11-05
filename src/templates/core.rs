use std::fmt::Display;

use axum_flash::Level;
use maud::{html, Markup, DOCTYPE};

pub type Msg<T> = (Level, T);
pub trait MsgIter<T: Display> = Iterator<Item = Msg<T>>;
pub trait MsgIterable<T: Display> = IntoIterator<Item = Msg<T>>;

pub fn layout<T: Display>(content: Markup, msgs: impl MsgIterable<T>) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            (head("Contacts"))
            body hx-ext="response-targets"{
                h1 {"Contact App"}
                h2 {"A HTMX Demo"}
                div #flashes {
                    (dismissible_alerts(msgs.into_iter()))
                }
                hr;
                (content)
                script src="https://cdn.jsdelivr.net/npm/bootstrap@5.3.2/dist/js/bootstrap.bundle.min.js"
                    integrity="sha384-C6RzsynM9kWDrMNeT87bh95OGNyZPhcTNXj1NW7RuBCsyN/o0jlpcV8Qyq46cDfL"
                    crossorigin="anonymous"{}

            }
        }
    }
}
fn head(title: &str) -> Markup {
    html! {
        head {
        meta charset="UTF-8";
            title {(title)}
            link rel="stylesheet"
                href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.2/dist/css/bootstrap.min.css"
                integrity="sha384-T3c6CoIi6uLrA9TneNEoa7RxnatzjcDSCmG1MXxSR1GAsXEV/Dwwykc2MPK8M2HN"
                crossorigin="anonymous";
            script
                src="https://unpkg.com/htmx.org@1.9.5"
                integrity="sha384-xcuj3WpfgjlKF+FXhSQFQ0ZNr39ln+hwjN3npfM9VBnUskLolQAcN80McRIVOPuO"
                crossorigin="anonymous"{}
            script src="https://unpkg.com/htmx.org@1.9.3/dist/ext/response-targets.js" {}
            script src="https://unpkg.com/hyperscript.org@0.9.12" {}
            style {
                "body {padding-left: 1em}"
                "td {padding-right: 1em}"
                "input {margin: 0.3em}"
                ".inline-err {padding: 0.3em 1em}"
            }
        }
    }
}

fn lvl2alert_class(lvl: &Level) -> String {
    let msg_type = match lvl {
        Level::Debug => "danger",
        Level::Info => "info",
        Level::Success => "success",
        Level::Warning => "warning",
        Level::Error => "danger",
    };
    format!("alert-{msg_type}")
}
pub fn inline_msg<T: Display>(msg: Msg<T>) -> Markup {
    let alert_class = lvl2alert_class(&msg.0);
    html! {
        span.alert.inline-err.(alert_class) {
            (msg.1)
        }
    }
}

pub fn dismissible_alerts<T: Display>(msgs: impl MsgIterable<T>) -> Markup {
    html! {
        @for (lvl, msg) in msgs{
            @let inner = html!{
                (msg)
                button.btn-close type="button" data-bs-dismiss="alert" aria-label="Close" {}
            };
            @let alert_class = lvl2alert_class(&lvl);
            div.alert.alert-dismissible.fade.show.(alert_class) role="alert"{
                    (inner) {}
            }

        }
    }
}
