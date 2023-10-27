use maud::{html, Markup};
use std::fmt::Display;

use crate::templates::{layout, MsgIterable};

pub fn new_user_template<T: Display>(
    msgs: impl MsgIterable<T>,
    email_feedback: Option<&str>,
    password_feedback: Option<&str>,
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
                    (password_input(password_feedback))
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
                    span.alert.alert-danger.inline-err hidden #repeat-nok role="alert" {
                        "Passwords do not match."
                    }
                }
                button { "save" }
            }
        }
    };
    layout(content, msgs)
}
fn email_input(init_value: &str, validation_url: &str, error_msg: Option<&str>) -> Markup {
    html! {
                        input #email
                            name="email"
                            type="email"
                            placeholder="name@example.org"
                            value=(init_value)
                            hx-get=(validation_url)
                            hx-params="*"
                            hx-trigger="change, keyup delay:350ms changed"
                            hx-target="next span"
                            hx-swap="outerHTML";
                        @if let Some(e) = error_msg {
                            span.alert.alert-danger role="alert" {
                                (e)
                            }
                        }
                        @else {
                            span {}
                        }

    }
}
fn password_input(error_msg: Option<&str>) -> Markup {
    html! {
                        input #password
                            name="password"
                            type="password"
                            hx-get="./password/validate"
                            hx-params="*"
                            hx-trigger="change, keyup delay:350ms changed"
                            hx-target="next span"
                            hx-swap="outerHTML"
                            _="on change or keyup debounced at 350ms
                                send newpass to #confirm-password";
                        @if let Some(e) = error_msg {
                            span.alert.alert-danger role="alert" {
                                (e)
                            }
                        }
                        @else {
                            span {}
                        }

    }
}
