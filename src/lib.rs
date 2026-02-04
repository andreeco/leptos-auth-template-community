include!(concat!(env!("OUT_DIR"), "/i18n/mod.rs"));
pub use i18n::*;
pub mod app;
pub mod components;
pub mod pages;
pub use app::{shell, App}; // if needed for SSR setup

pub mod auth;
pub mod auth_state;
pub mod csrf;
pub mod state;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::App;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
