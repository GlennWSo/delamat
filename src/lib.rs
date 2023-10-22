#![feature(trait_alias)]

use axum::extract::FromRef;
use axum_flash::Key;
use db::DB;

pub mod auth;
pub mod db;
pub mod email;
pub mod templates;

#[derive(Clone)]
pub struct AppState {
    pub db: DB,
    flash_config: axum_flash::Config,
}

impl AppState {
    pub fn new(db: DB) -> Self {
        let flash_config = axum_flash::Config::new(Key::generate());
        Self { db, flash_config }
    }
}
impl FromRef<AppState> for axum_flash::Config {
    fn from_ref(state: &AppState) -> Self {
        state.flash_config.clone()
    }
}
