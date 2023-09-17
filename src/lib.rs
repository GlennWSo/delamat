mod db;
mod templates;

// pub use contact::{get_all_contacts, Contact};

use askama::Template;
// bring trait in scope
pub use db::{Contact, DB};

type Messages<'a> = &'a [&'a str];

#[derive(Template)]
#[template(path = "layout.html")]
struct BaseTemplate<'a> {
    // field name should match the variable name
    messages: Messages<'a>,
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct ContactsTemplate<'a> {
    pub messages: Messages<'a>,
    pub contacts: Vec<Contact>,
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
