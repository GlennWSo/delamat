mod core;
use core::layout;

use crate::{db::Contact};
// use askama::Template;

use maud::{html, Markup};

use self::core::MsgIterable;

pub fn new_contact<'a>(
    name: &str,
    email: &str,
    email_error: Option<&str>,
    flashes: impl MsgIterable<'a>,
) -> Markup {
    let content = html! {
        div #main {
            form action="/contacts/new" method="post" {
                legend {"Contact Values"}
                p {
                    label for="name" {"Name"}
                    input #name name="name" placeholder="Name Surname" value=(name);
                }
                p {
                    label for="email"{
                        "Email"
                    }
                    input #email
                        name="email"
                        type="email"
                        placeholder="name@example.org"
                        value=(email)
                        hx-get="/contacts/email"
                        hx-params="*"
                        hx-trigger="change, keyup delay:350ms changed"
                        hx-target="next span"
                        hx-swap="outerHTML";
                    @if let Some(e) = email_error {
                        span.alert.alert-danger role="alert" {
                            (e)
                        }
                    }
                    @else {
                        span {}
                    }
                }
                button {"Saveasdasd"}

            }
        }
        p {
            a href="/contacts"{
                "Back"
            }
        }
    };

    layout(dbg!(content), flashes)
}

pub fn edit_contact<'a>(
    contact: &Contact,
    flashes: impl MsgIterable<'a>,
    email_error: Option<&str>,
) -> Markup {
    let content = html! {
        div #main {
        p {
            a href={"contacts/"(contact.id)} { "View" }
            a href="/contacts" {" back"}
        }
        h1 {"Editing " (contact.name)}
        form action={"/contacts/"(contact.id)"/edit"} method="post" {
            fieldset {
                legend {"Contact values"}
                p {
                    label for="name" { "Name" }
                    input #name name="name" type="text" placeholder="name" value=(contact.name);
                }
                p {
                    label for="email" { "Email"}
                    input #email
                        name="email"
                        type="email"
                        placeholder="name@example.org"
                        value=(contact.email)
                        hx-get="/contacts/email"
                        hx-params="*"
                        hx-vals=(format!("'id': '{}'", contact.id))
                        hx-trigger="change, keyup delay:350ms changed"
                        hx-target="next span"
                        hx-swap="outerHTML";
                    @if let Some(e) = email_error {
                        span.alert.alert-danger role="alert" {
                            (e)
                        }
                    }
                    @else {
                        span {}
                    }
                }
                button { "save" }
            }
            hr;
            button
                hx-delete={"contacts/"(contact.id)}
                hx-confirm="Are you sure?"
                hx-push-url="true"
                hx-target="body" {
                    "Delete Contact"
            }
        }
    }};

    layout(content, flashes)
}

pub fn contact_details<'a>(flashes: impl MsgIterable<'a>, contact: &Contact) -> Markup {
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
    layout(content, flashes)
}
pub fn contact_list<'a>(
    flashes: impl MsgIterable<'a>,
    contacts: &[Contact],
    page: u32,
    more_pages: bool,
) -> Markup {
    let search_form = html! {
            form #tool-bar action="/contacts" method="get" {
                label for="search" {
                    "Search Term"
                }
                input.search type="search" name="name" value="";
                input type="submit" value="Search";
            }

    };

    let table = html! {
        table {
            thead {
                th {"Name"}
                th {"Email"}
                th {"Links"}
            }
        @for c in contacts{
            tr {
                    td{(c.name)}
                    td{(c.email)}
                    td{
                      a href={"/contacts/"(c.id)} {"View"}
                      a href={"/contacts/"(c.id)"/edit"} {"Edit"}
                      a href=""
                        hx-confirm="Are you sure?"
                        hx-delete={"/contacts/"(c.id)}
                        hx-target="body"{
                        "Delete"
                      }
                    }
                }

            }
        }
    };

    let pager = html! {
        span style="float: right"{
            div #pager {
                @if page > 1 {
                    a href={"/contacts?page="((page - 1))} {"Previous"}
                }
                " ("(page)") "
                @if more_pages {
                    a href={"/contacts?page="((page+1))} {"Next"}
                }
            }
        }
    };
    let content = html! {
        div #main {
            (search_form)
            (table)
            (pager)
            div {
                a href="/contacts/new" {"Create New"}
                ", "
                a href="/contacts/download" hx-boost="false" {
                    "Download Contacts"
                }
            }
        }
    };
    layout(content, flashes)
}
