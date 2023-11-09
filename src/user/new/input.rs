use std::fmt::Display;

use axum_flash::Level;
use futures_util::future::Select;
use maud::{html, Markup};

use crate::{db::DB, templates::inline_msg};

pub fn validate_char(c: &char) -> bool {
    if c.is_alphabetic() {
        return true;
    }
    if c.is_numeric() {
        return true;
    }
    if "_-".contains([*c]) {
        return true;
    }
    false
}
pub trait Feedback<E: Display> {
    const CFG: &'static InputAttributes;
    async fn validate(&self, db: &DB) -> Option<E>;
    fn into_value(self) -> Box<str>;
    async fn into_input(self, db: &DB) -> InputField<E>
    where
        Self: Sized,
    {
        match self.validate(db).await {
            Some(error) => InputField::with_state(
                Self::CFG,
                InputState::Invalid {
                    value: self.into_value(),
                    error,
                },
            ),
            _ => InputField::with_state(Self::CFG, InputState::Valid(self.into_value())),
        }
    }
}
pub struct InputAttributes {
    pub label: &'static str,
    pub name: &'static str,
    /// used for attribute 'type'='kind'
    pub kind: &'static str,
    pub placeholder: &'static str,
    pub validate_api: &'static str,
}

pub enum InputState<E: Display> {
    Init,
    Invalid { value: Box<str>, error: E },
    Valid(Box<str>),
}

pub struct InputField<E: Display> {
    state: InputState<E>,
    cfg: &'static InputAttributes,
}

impl<E: Display> InputField<E> {
    pub fn new(cfg: &'static InputAttributes) -> Self {
        Self {
            state: InputState::Init,
            cfg,
        }
    }
    pub fn with_state(cfg: &'static InputAttributes, state: InputState<E>) -> Self {
        Self { state, cfg }
    }
    pub fn into_state(self) -> InputState<E> {
        self.state
    }
    fn error(&self) -> Option<&E> {
        match &self.state {
            InputState::Invalid { value, error } => Some(&error),
            _ => None,
        }
    }
    fn style(&self) -> &'static str {
        match self.state {
            InputState::Init => "",
            InputState::Invalid { .. } => "box-shadow: 0 0 3px #CC0000",
            InputState::Valid(_) => "box-shadow: 0 0 3px #36cc00;",
        }
    }
    pub fn into_markup(self) -> Markup {
        let cfg = self.cfg;
        let style = self.style();
        let (value, error) = match self.state {
            InputState::Init => ("".into(), None),
            InputState::Invalid { value, error } => (value, Some(error)),
            InputState::Valid(value) => (value, None),
        };
        html! {
            div.input_field hx-target="this" {
                label for=(cfg.name) { (cfg.label) }
                input #name
                    name=(cfg.name)
                    type=(cfg.kind)
                    placeholder=(cfg.placeholder)
                    hx-post=(cfg.validate_api)
                    hx-params="*"
                    hx-trigger="change, keyup delay:350ms changed, htmx:validation:validate"
                    value=(value)
                    style=(style) {}
                @match error {
                     Some(e)=> (inline_msg((Level::Error, e))),
                    _ => span {},
                }
            }

        }
    }
}
