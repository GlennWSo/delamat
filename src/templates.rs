use crate::db::Contact;
use askama::Template;
use axum_flash::IncomingFlashes;
use maud::{html, Markup, PreEscaped, DOCTYPE};

type Messages<'a> = &'a [&'a str];

pub fn hello_world(name: Option<Box<str>>) -> Markup {
    html! {
        h2 {"Hello, " (name.unwrap_or("World!".into()))}
    }
}

#[derive(Template)]
#[template(path = "layout.html")]
struct BaseTemplate<'a> {
    // field name should match the variable name
    messages: Messages<'a>,
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct ContactsTemplate<'a> {
    pub page: u32,
    pub messages: Messages<'a>,
    pub contacts: &'a [Contact],
    pub more_pages: bool,
}

#[derive(Template)]
#[template(path = "new.html")]
pub struct NewTemplate<'a> {
    pub messages: Messages<'a>,
    pub name: &'a str,
    pub email: &'a str,
    pub email_error: Option<String>,
}

impl<'a> NewTemplate<'a> {
    pub fn new(name: &'a str, email: &'a str, email_error: Option<String>) -> Self {
        Self {
            name,
            email,
            messages: &[],
            email_error,
        }
    }
}

#[derive(Template)]
#[template(path = "edit.html")]
pub struct EditTemplate<'a> {
    pub messages: &'a [&'a str],
    pub email_error: Option<String>,
    pub contact: Contact,
}

impl<'a> EditTemplate<'a> {
    pub fn new(messages: &'a [&'a str], email_error: Option<String>, contact: Contact) -> Self {
        Self {
            messages,
            email_error,
            contact,
        }
    }
}

fn head(title: &str) -> Markup {
    html! {
        head {
        meta charset="UTF-8";
            title {(title)}
            link rel="stylesheet"
                href="https://cdn.jsdelivr.net/npm/bootstrap@4.0.0/dist/css/bootstrap.min.css"
                integrity="sha384-Gn5384xqQ1aoWXA+058RXPxPg6fy4IWvTNh0E263XmFcJlSAwiGgFAW/dAiS6JXm"
                crossorigin="anonymous";
            script
                src="https://unpkg.com/htmx.org@1.9.5"
                integrity="sha384-xcuj3WpfgjlKF+FXhSQFQ0ZNr39ln+hwjN3npfM9VBnUskLolQAcN80McRIVOPuO"
                crossorigin="anonymous"{}
            style {
                "\n\tbody {padding-left: 1em}"
                "\n\ttd {padding-right: 1em}"
            }
        }
    }
}

/// flash msg template
fn flashy_flash(flashes: &IncomingFlashes) -> Markup {
    html! {
        @for (lvl, msg) in flashes.iter(){
            @match lvl {
                axum_flash::Level::Debug => {},
                axum_flash::Level::Info => {},
                axum_flash::Level::Warning => {},
                axum_flash::Level::Error => {},
                axum_flash::Level::Success => {
                    div.alert.alert-success.alert-dismissible.fade.show {
                        (msg)
                        button #close type="button" data-dismiss="alert" aria-label="Close" {
                            span aria-hidden="true" {
                            "&times;"
                            }
                        }
                    }
                },
            }

        }
    }
}

/// content should wrap it self with something
fn layout(flashes: &IncomingFlashes, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            (head("Contacts"))
            body{
                h1 {"Contact App"}
                h2 {"A HTMX Demo"}
                (flashy_flash(flashes))
                hr;
                (content)
                script src="https://code.jquery.com/jquery-3.2.1.slim.min.js" integrity="sha384-KJ3o2DKtIkvYIK3UENzmM7KCkRr/rE9/Qpg6aAZGJwFDMVNA/GpGFF93hXpG5KkN" crossorigin="anonymous";
                script src="https://cdn.jsdelivr.net/npm/popper.js@1.12.9/dist/umd/popper.min.js" integrity="sha384-ApNbgh9B+Y1QKtv3Rn7W3mgPxhU9K/ScQsAP7hUibX39j7fakFPskvXusvfa0b4Q" crossorigin="anonymous";
                script src="https://cdn.jsdelivr.net/npm/bootstrap@4.0.0/dist/js/bootstrap.min.js" integrity="sha384-JZR6Spejh4U02d8jOt6vLEHfe/JQGiRRSQQxSfFWpi1MquVdAyjUar5+76PVCmYl" crossorigin="anonymous";

            }
        }
    }
}

pub fn contact_details(flashes: &IncomingFlashes, contact: Contact) -> Markup {
    let content = html! {
        div #main{
            p {
                a href={"/contacts/"(contact.id)"/edit"} {"Edit"}
                a href={"/contacts"} {"Back"}
            }
            h1 {
                (contact.name)
            }
            div {
                div {"email"(contact.email)}
            }
        }
    };
    layout(&flashes, content)
}

#[derive(Template)]
#[template(path = "view.html")]
pub struct ViewTemplate<'a> {
    messages: Vec<&'a str>,
    contact: Contact,
}
impl<'a> ViewTemplate<'a> {
    pub fn with_messages(msgs: &[&'a str], contact: Contact) -> Self {
        Self {
            messages: msgs.into(),
            contact,
        }
    }
}
