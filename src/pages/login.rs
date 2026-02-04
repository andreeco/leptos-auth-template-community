use crate::auth_state::AuthState;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

#[server(LoginUser)]
pub async fn login_user(
    username: String,
    password: String,
    csrf: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    crate::csrf::require_csrf(&csrf).await?;

    use crate::auth::{AuthSession, Credentials};
    use axum::Extension;
    use http::StatusCode;
    use leptos_axum::ResponseOptions;
    use tower_sessions::Session;

    let response = expect_context::<ResponseOptions>();

    let Extension(mut auth): Extension<AuthSession> = leptos_axum::extract().await?;
    let Extension(session): Extension<Session> = leptos_axum::extract().await?;

    let creds = Credentials { username, password };

    match auth.authenticate(creds).await {
        Ok(Some(user)) => {
            // Strong: rotate session id before writing auth into session
            if let Err(e) = session.cycle_id().await {
                eprintln!("session.cycle_id failed: {e}");
                response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
                return Err(ServerFnError::ServerError("Session error".into()));
            }

            if let Err(e) = auth.login(&user).await {
                eprintln!("auth.login failed: {e}");
                response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
                return Err(ServerFnError::ServerError("Internal error".into()));
            }

            // Strong: rotate CSRF after login
            if let Err(e) = crate::csrf::rotate_csrf_token().await {
                eprintln!("rotate_csrf_token failed: {e}");
                response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
                return Err(ServerFnError::ServerError("CSRF error".into()));
            }

            leptos_axum::redirect("/protected");
            Ok(())
        }
        Ok(None) => {
            response.set_status(StatusCode::UNAUTHORIZED);
            Err(ServerFnError::ServerError("Invalid credentials".into()))
        }
        Err(e) => {
            eprintln!("auth.authenticate failed: {e}");
            response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
            Err(ServerFnError::ServerError("Internal error".into()))
        }
    }
}

#[component]
pub fn LoginPage() -> impl IntoView {
    let login_action = ServerAction::<LoginUser>::new();
    let auth = expect_context::<AuthState>();
    let navigate = use_navigate();

    // CSRF token signal
    let csrf_sig = use_context::<crate::csrf::CsrfContext>()
        .map(|c| c.0)
        .unwrap_or_else(|| RwSignal::new(None::<String>));

    let csrf_value = move || csrf_sig.read().clone().unwrap_or_default();
    let csrf_ready = move || csrf_sig.read().is_some();

    // CSRF refresh trigger provided by App()
    let csrf_refresh = expect_context::<RwSignal<()>>();

    Effect::new(move |_| {
        if let Some(Ok(())) = login_action.value().get() {
            auth.set_ready.set(false);

            // refresh CSRF after server rotated it
            csrf_refresh.set(());

            let navigate = navigate.clone();
            leptos::task::spawn_local({
                let auth = auth; // Copy
                async move {
                    if let Ok(snap) = crate::auth_state::auth_snapshot().await {
                        auth.set_user.set(snap.user);
                        auth.set_permissions.set(snap.permissions);
                        auth.set_ready.set(true);
                        navigate("/protected", Default::default());
                    }
                }
            });
        }
    });

    view! {
        <h1>"Login"</h1>

        {move || login_action.value().get()
            .and_then(|res| res.err())
            .map(|err| view! { <div class="error">{err.to_string()}</div> })
        }

        <ActionForm action=login_action>
            <input type="hidden" name="csrf" value=csrf_value />

            <input type="text" name="username" placeholder="username" value="admin"/>
            <input type="password" name="password" placeholder="password" value="password"/>

            <button
                type="submit"
                disabled=move || !csrf_ready() || login_action.pending().get()
            >
                {move || if login_action.pending().get() { "Logging in..." } else { "Log In" }}
            </button>
        </ActionForm>
    }
}
