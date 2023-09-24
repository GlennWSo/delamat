use axum_flash::IncomingFlashes;
use maud::{html, Markup, DOCTYPE};

///should wrap it self with something
pub fn layout(flashes: &IncomingFlashes, content: Markup) -> Markup {
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
                script src="https://code.jquery.com/jquery-3.2.1.slim.min.js"
                    integrity="sha384-KJ3o2DKtIkvYIK3UENzmM7KCkRr/rE9/Qpg6aAZGJwFDMVNA/GpGFF93hXpG5KkN"
                    crossorigin="anonymous"{}
                script src="https://cdn.jsdelivr.net/npm/popper.js@1.12.9/dist/umd/popper.min.js"
                    integrity="sha384-ApNbgh9B+Y1QKtv3Rn7W3mgPxhU9K/ScQsAP7hUibX39j7fakFPskvXusvfa0b4Q"
                    crossorigin="anonymous"{}
                script src="https://cdn.jsdelivr.net/npm/bootstrap@4.0.0/dist/js/bootstrap.min.js"
                    integrity="sha384-JZR6Spejh4U02d8jOt6vLEHfe/JQGiRRSQQxSfFWpi1MquVdAyjUar5+76PVCmYl"
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

/// content /// flash msg template
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
