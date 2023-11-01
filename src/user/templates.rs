use maud::{html, Markup};
use std::fmt::Display;

use crate::templates::{layout, MsgIterable};

use super::NewUserInput;

pub fn new_template<T: Display>(
    msgs: impl MsgIterable<T>,
    email_feedback: Option<T>,
    password_feedback: Option<T>,
    input: NewUserInput,
) -> Markup {
    let content = html! {
        h2 {"Create a Account"}
        form hx-post="/user/new" hx-target="closest <body/>" "hx-target-500"="#flashes" {
            fieldset {
                p {
                    label for="name" { "Name" }
                    input #name name="name" type="text" placeholder="your alias" value=(input.name);
                }
                p {
                    label for="email" { "email" }
                    (email_input(&input.email, "./email/validate", email_feedback))
                }
                p {
                    label for="password" { "Password" }
                    (password_input(&input.password, password_feedback))
                }
                p {
                    label for="confirm-password" { "Confirm Password" }
                    input #confirm-password type="password" _="
                        on newpass or change or keyup debounced at 350ms  
                        if my value equals #password.value and my value is not ''
                            remove @hidden from #repeat-ok
                            then add @hidden to #repeat-nok
                            then send confirm(ok: true) to next <button/>
                        else if my.value is not ''
                            then add @hidden to #repeat-ok
                            then remove @hidden from #repeat-nok
                            then send confirm(ok: false) to next <button/>
                        else
                            send confirm(ok: false) to next <button/>
                    "
                    ;
                    span #repeat-ok hidden {"✅"}
                    span.alert.alert-danger.inline-err hidden #repeat-nok role="alert" {
                        "Passwords do not match."
                    }
                }
                button disabled _="
                    on load set :feedback to {password: false, email: false, confirm: false}
                        then add @disabled on me
                    end
                    
                    def update_me()
                        if :feedback.password and :feedback.email and :feedback.confirm
                            remove @disabled
                        else
                            add @disabled
                    end
                
                    on password(ok) 
                        set :feedback.password to ok then update_me()
                    end
                    on email(ok) 
                        set :feedback.email to ok then update_me()
                    end
                    on confirm(ok) 
                        set :feedback.confirm to ok then update_me()
                    end
                    "
                    { "save" }

            }
        }
    };
    layout(content, msgs)
}
fn email_input<T: Display>(init_value: &str, validation_url: &str, error_msg: Option<T>) -> Markup {
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
                            span.alert.alert-danger.inline-err role="alert" {
                                (e)
                            }
                        }
                        @else {
                            span {}
                        }

    }
}
fn password_input<T: Display>(init_value: &str, error_msg: Option<T>) -> Markup {
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
                            span.alert.alert-danger role="alert" {
                                (e)
                            }
                        }
                        @else {
                            span {}
                        }

    }
}
