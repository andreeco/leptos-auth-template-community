#![recursion_limit = "1024"]

include!(concat!(env!("OUT_DIR"), "/i18n/mod.rs"));
pub use i18n::*;
pub mod app;
pub mod components;
pub mod pages;
pub mod features;
pub mod contexts;
pub mod i18n_utils;
pub mod state;

#[cfg(feature = "ssr")]
pub use db::entities;

pub use app::{shell, App};

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_islands();
}
