use axum_flash::IncomingFlashes;
use maud::{html, Markup, DOCTYPE};

///should wrap it self with something
pub fn layout(content: Markup, flashes: &IncomingFlashes) -> Markup {
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
                    div.alert.alert-success.alert-dismissible.fade.show role="alert"{
                        (msg)
                        button.btn-close type="button" data-bs-dismiss="alert" aria-label="Close" {
                            // span aria-hidden="true" {r#"&times;"#}
                        }
                    }
                },
            }

        }
    }
}
