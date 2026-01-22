use leptos::prelude::*;

#[cfg(feature = "ssr")]
use crate::auth::AuthSession; // your axum-login alias

#[server]
pub async fn is_logged_in() -> Result<bool, ServerFnError> {
    let auth: AuthSession = leptos_axum::extract().await?;
    Ok(auth.user.is_some())
}

#[component]
pub fn ProtectedLeptos() -> impl IntoView {
    view! {
        <h1>Protected Leptos Page</h1>
        <p>You are logged in (checked in Leptos router).</p>
        <p>You can manually change the signal in app.rs.</p>
        <a href="/logout">Logout (server)</a>
    }
}
