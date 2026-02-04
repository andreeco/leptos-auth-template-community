use crate::auth_state::AuthState;
use crate::i18n::*;
use leptos::prelude::*;

#[server(prefix = "/api/secure")]
pub async fn admin_add_two(a: i32, b: i32) -> Result<i32, ServerFnError> {
    use crate::auth::{AuthSession, Role};
    use axum::Extension;

    let Extension(auth): Extension<AuthSession> = leptos_axum::extract().await?;

    let Some(user) = auth.user.clone() else {
        return Err(ServerFnError::ServerError("Not authenticated".into()));
    };

    if !user.roles.contains(&Role::Admin) {
        return Err(ServerFnError::ServerError("Forbidden".into()));
    }

    Ok(a + b)
}

#[component]
pub fn AdminPage() -> impl IntoView {
    let i18n = use_i18n();
    let auth = expect_context::<AuthState>();

    let result = RwSignal::new(None::<i32>);
    let error = RwSignal::new(None::<String>);

    let compute = move |_| {
        leptos::task::spawn_local(async move {
            match admin_add_two(21, 21).await {
                Ok(v) => {
                    result.set(Some(v));
                    error.set(None);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    result.set(None);
                }
            }
        });
    };

    view! {
        <h1>{t!(i18n, admin.title)}</h1>

        <Show
            when=move || auth.ready.get() && auth.is_admin()
            fallback=move || view! { <p>{t!(i18n, admin.forbidden)}</p> }
        >
            <p>{t!(i18n, admin.welcome)} " " {move || auth.username().unwrap_or_default()}</p>
            <p>{t!(i18n, admin.fake_note)}</p>

            <button on:click=compute>"Admin add 21 + 21 (server)!"</button>

            <Show when=move || result.get().is_some()>
                <p>{move || result.get().unwrap().to_string()}</p>
            </Show>

            <Show when=move || error.get().is_some()>
                <p style="color:red">{move || error.get().unwrap_or_default()}</p>
            </Show>
        </Show>
    }
}
