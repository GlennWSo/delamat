
use maud::{html, Markup};

use std::fmt::Display;

use crate::{
    templates::{MsgIterable},
};

pub fn email_input<T: Display>(
    init_value: &str,
    validation_url: &str,
    error_msg: Option<T>,
) -> Markup {
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
                            "hx-target-500"="next span"
                            "hx-target-406"="next span"
                            hx-swap="outerHTML";
                        @if let Some(e) = error_msg {
                            span.alert.alert-danger.inline-err role="alert" {
                                (e)
                            }
                        }
                        @else {
                            span {}
                        }

    }
}

pub fn password_input<T: Display>(init_value: &str, error_msg: Option<T>) -> Markup {
    html! {
        input #password
            name="password"
            type="password"
            value=(init_value)
            hx-get="./password/validate"
            hx-params="*"
            hx-trigger="change, keyup delay:350ms changed"
            hx-target="next span"
            hx-swap="outerHTML"
            _="on change or keyup debounced at 350ms
                send newpass to #confirm-password";
        @if let Some(e) = error_msg {
            span.alert.alert-danger.inline-err role="alert" {
                (e)
            }
        }
        @else {
            span {}
        }

    }
}
