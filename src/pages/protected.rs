use leptos::prelude::*;

// #[cfg(feature = "ssr")]
// use crate::auth::AuthSession; // your axum-login alias

#[server]
pub async fn is_logged_in() -> Result<bool, ServerFnError> {
    use crate::auth::AuthSession;

    let auth: AuthSession = leptos_axum::extract().await?;
    Ok(auth.user.is_some())
}

#[component]
pub fn Protected() -> impl IntoView {
    view! {
        <h1>Protected Page</h1>
        <p>You are logged.</p>
        <p>You can manually change the signal in app.rs.</p>
        <a href="/logout">Logout (server)</a>
    }
}
