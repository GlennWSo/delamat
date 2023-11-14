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
    const CFG: &'static Config;
    async fn validate(&self, db: &DB) -> Result<(), E>;
    fn into_value(self) -> Box<str>;
    async fn into_input(self, db: &DB) -> InputField<E>
    where
        Self: Sized,
    {
        match self.validate(db).await {
            Err(error) => InputField::with_state(
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

pub type Attributes<'a> = [(&'a str, &'a str)];

#[derive(Default)]
pub struct Config {
    pub label: &'static str,
    pub name: &'static str,
    /// used for type attr
    pub kind: Option<&'static str>,
    pub placeholder: Option<&'static str>,
    pub validate_api: Option<&'static str>,
    pub hyper_script: Option<&'static str>,
}

pub enum InputState<E: Display> {
    Init,
    Invalid { value: Box<str>, error: E },
    Valid(Box<str>),
}

pub struct InputField<E: Display> {
    state: InputState<E>,
    cfg: &'static Config,
}

impl<E: Display> InputField<E> {
    pub fn new(cfg: &'static Config) -> Self {
        Self {
            state: InputState::Init,
            cfg: cfg,
        }
    }
    pub fn with_state(cfg: &'static Config, state: InputState<E>) -> Self {
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
        enum State<E: Display> {
            Init,
            Valid,
            Invalid(E),
        }
        let (value, state, script) = match self.state {
            InputState::Init => ("".into(), State::Init, None),
            InputState::Invalid { value, error } => (
                value,
                State::Invalid(error),
                Some(
                    "on load 
                    add .nok to previous <input/> 
                    then remove .ok from previous <input/>",
                ),
            ),
            InputState::Valid(value) => (
                value,
                State::Valid,
                Some(
                    "on load 
                    add .ok to previous <input/> 
                    then remove .nok from previous <input/>",
                ),
            ),
        };
        html! {
            div.input_field   {
                label for=(cfg.name) { (cfg.label) }
                input #(cfg.name)
                    hx-target="next <span/>"
                    hx-select="#feedback"
                    hx-swap="innerHTML"
                    name=(cfg.name)
                    type=[cfg.kind]
                    placeholder=[cfg.placeholder]
                    value=(value)
                    hx-post=[cfg.validate_api]
                    style=(style)
                    hx-params="*"
                    hx-trigger="change, keyup delay:350ms changed, htmx:validation:validate"
                    _=[cfg.hyper_script]
                    {}
                span #feedback _=[script]{
                    @match state  {
                        State::Invalid(e) => (inline_msg((Level::Error, e))),
                        // State::Valid => "✅",
                        _ => "",
                    }
                }
            }

        }
    }
}
