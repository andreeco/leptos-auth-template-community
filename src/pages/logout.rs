use crate::auth_state::AuthState;
use crate::i18n::*;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use std::collections::HashSet;

#[server(LogoutUser)]
pub async fn logout_user(csrf: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    crate::csrf::require_csrf(&csrf).await?;

    use crate::auth::AuthSession;
    use axum::Extension;
    use http::StatusCode;
    use leptos_axum::ResponseOptions;
    use tower_sessions::Session;

    let response = expect_context::<ResponseOptions>();

    let Extension(mut auth): Extension<AuthSession> = leptos_axum::extract().await?;
    let Extension(session): Extension<Session> = leptos_axum::extract().await?;

    if let Err(e) = auth.logout().await {
        eprintln!("logout failed: {e}");
        response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
        return Err(ServerFnError::ServerError("Internal error".into()));
    }

    if let Err(e) = session.cycle_id().await {
        eprintln!("session.cycle_id failed on logout: {e}");
        response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
        return Err(ServerFnError::ServerError("Session error".into()));
    }

    if let Err(e) = crate::csrf::rotate_csrf_token().await {
        eprintln!("rotate_csrf_token failed on logout: {e}");
        response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
        return Err(ServerFnError::ServerError("CSRF error".into()));
    }

    leptos_axum::redirect("/");
    Ok(())
}

#[component]
pub fn LogoutPage() -> impl IntoView {
    let logout_action = ServerAction::<LogoutUser>::new();
    let auth = expect_context::<AuthState>();
    let navigate = use_navigate();
    let i18n = use_i18n();

    let csrf_sig = use_context::<crate::csrf::CsrfContext>()
        .map(|c| c.0)
        .unwrap_or_else(|| RwSignal::new(None::<String>));

    let csrf_value = move || csrf_sig.read().clone().unwrap_or_default();
    let csrf_ready = move || csrf_sig.read().is_some();

    // CSRF refresh trigger provided by App()
    let csrf_refresh = expect_context::<RwSignal<()>>();

    Effect::new(move |_| {
        if let Some(Ok(())) = logout_action.value().get() {
            auth.set_user.set(None);
            auth.set_permissions.set(HashSet::new());
            auth.set_ready.set(true);

            // refresh CSRF after server rotated it
            csrf_refresh.set(());

            navigate("/login", Default::default());
        }
    });

    view! {
        <h1>{t!(i18n, logout.title)}</h1>

        <Show
            when=move || !logout_action.pending().get()
            fallback=move || view! { <p>{t!(i18n, logout.logging_out)}</p> }
        >
            <p>{t!(i18n, logout.confirm)}</p>

            <ActionForm action=logout_action>
                <input type="hidden" name="csrf" value=csrf_value />
                <input
                    type="submit"
                    value="Logout"
                    disabled=move || !csrf_ready() || logout_action.pending().get()
                />
            </ActionForm>

            <p>
                <a href="/">{t!(i18n, logout.cancel)}</a>
            </p>

            {move || logout_action.value().get()
                .and_then(|r| r.err())
                .map(|e| view! { <p class="error">{e.to_string()}</p> })
            }
        </Show>
    }
}
