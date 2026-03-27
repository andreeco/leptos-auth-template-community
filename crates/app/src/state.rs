#![cfg(feature = "ssr")]

use axum::extract::FromRef;
use leptos::prelude::LeptosOptions;
use leptos_axum::AxumRouteListing;
use sea_orm::DatabaseConnection;
use webauthn_rs::Webauthn;

#[derive(FromRef, Debug, Clone)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub routes: Vec<AxumRouteListing>,
    pub db: DatabaseConnection,
    pub webauthn: Webauthn,
}
