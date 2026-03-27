use crate::account::{get_credential, webauthn_login_finish, webauthn_login_start};
use crate::auth_state::AuthState;
use crate::i18n::*;
use crate::i18n_paths::lp;
use leptos::prelude::*;
use leptos_router::components::A;
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
    use std::time::{SystemTime, UNIX_EPOCH};
    use tower_sessions::Session;

    const LOGIN_FAIL_COUNT_KEY: &str = "login_fail_count";
    const LOGIN_LOCKED_UNTIL_KEY: &str = "login_locked_until";
    const MAX_LOGIN_ATTEMPTS: u32 = 5;
    const LOCKOUT_SECONDS: i64 = 300;

    let response = expect_context::<ResponseOptions>();

    let Extension(mut auth): Extension<AuthSession> = leptos_axum::extract().await?;
    let Extension(session): Extension<Session> = leptos_axum::extract().await?;


    let now_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let locked_until = session
        .get::<i64>(LOGIN_LOCKED_UNTIL_KEY)
        .await
        .map_err(|e| {
            eprintln!("session.get({LOGIN_LOCKED_UNTIL_KEY}) failed: {e}");
            response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
            ServerFnError::new("error_session")
        })?
        .unwrap_or(0);

    if locked_until > now_epoch {
        response.set_status(StatusCode::TOO_MANY_REQUESTS);
        return Err(ServerFnError::ServerError("error_invalid_credentials".into()));
    }

    let creds = Credentials { username, password };

    match auth.authenticate(creds).await {
        Ok(Some(user)) => {
            if let Err(e) = session.cycle_id().await {
                eprintln!("session.cycle_id failed: {e}");
                response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
                return Err(ServerFnError::ServerError("error_session".into()));
            }

            if let Err(e) = auth.login(&user).await {
                eprintln!("auth.login failed: {e}");
                response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
                return Err(ServerFnError::ServerError("error_internal".into()));
            }

            if let Err(e) = session.insert(LOGIN_FAIL_COUNT_KEY, 0u32).await {
                eprintln!("session.insert({LOGIN_FAIL_COUNT_KEY}) failed: {e}");
                response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
                return Err(ServerFnError::ServerError("error_session".into()));
            }

            if let Err(e) = session.remove::<i64>(LOGIN_LOCKED_UNTIL_KEY).await {
                eprintln!("session.remove({LOGIN_LOCKED_UNTIL_KEY}) failed: {e}");
                response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
                return Err(ServerFnError::ServerError("error_session".into()));
            }

            if let Err(e) = crate::csrf::rotate_csrf_token().await {
                eprintln!("rotate_csrf_token failed: {e}");
                response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
                return Err(ServerFnError::ServerError("error_security_token".into()));
            }

            let redirect_target = {
                let locale: Locale = leptos_i18n::locale::resolve_locale();

                if user.password_reset_required {
                    lp(locale, td_string!(locale, routes.account_password_path))
                } else {
                    lp(locale, td_string!(locale, routes.protected_path))
                }
            };

            leptos_axum::redirect(&redirect_target);
            Ok(())
        }
        Ok(None) => {
            let fail_count = session
                .get::<u32>(LOGIN_FAIL_COUNT_KEY)
                .await
                .map_err(|e| {
                    eprintln!("session.get({LOGIN_FAIL_COUNT_KEY}) failed: {e}");
                    response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
                    ServerFnError::new("error_session")
                })?
                .unwrap_or(0);

            let new_fail_count = fail_count.saturating_add(1);

            if new_fail_count >= MAX_LOGIN_ATTEMPTS {
                let lock_until = now_epoch + LOCKOUT_SECONDS;
                if let Err(e) = session.insert(LOGIN_LOCKED_UNTIL_KEY, lock_until).await {
                    eprintln!("session.insert({LOGIN_LOCKED_UNTIL_KEY}) failed: {e}");
                    response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
                    return Err(ServerFnError::ServerError("error_session".into()));
                }
                if let Err(e) = session.insert(LOGIN_FAIL_COUNT_KEY, 0u32).await {
                    eprintln!("session.insert({LOGIN_FAIL_COUNT_KEY}) failed: {e}");
                    response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
                    return Err(ServerFnError::ServerError("error_session".into()));
                }

                response.set_status(StatusCode::TOO_MANY_REQUESTS);
                return Err(ServerFnError::ServerError("error_invalid_credentials".into()));
            }

            if let Err(e) = session.insert(LOGIN_FAIL_COUNT_KEY, new_fail_count).await {
                eprintln!("session.insert({LOGIN_FAIL_COUNT_KEY}) failed: {e}");
                response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
                return Err(ServerFnError::ServerError("error_session".into()));
            }

            response.set_status(StatusCode::UNAUTHORIZED);
            Err(ServerFnError::ServerError("error_invalid_credentials".into()))
        }
        Err(e) => {
            eprintln!("auth.authenticate failed: {e}");
            response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
            Err(ServerFnError::ServerError("error_internal".into()))
        }
    }
}

fn normalize_login_error_key(raw: &str) -> &'static str {
    let lower = raw.to_lowercase();

    if raw == "error_invalid_credentials" || lower.contains("invalid credentials") {
        return "error_invalid_credentials";
    }
    if raw == "error_missing_csrf" || lower.contains("csrf") {
        return "error_missing_csrf";
    }
    if raw == "error_security_token" || lower.contains("security token") {
        return "error_security_token";
    }
    if raw == "error_session" || lower.contains("session error") {
        return "error_session";
    }
    if raw == "error_session_refresh_failed" || lower.contains("session refresh failed") {
        return "error_session_refresh_failed";
    }
    if raw == "error_passkey_failed" || lower.contains("passkey") {
        return "error_passkey_failed";
    }
    if raw == "error_passkey_session_refresh_failed"
        || lower.contains("passkey login succeeded but session refresh failed")
    {
        return "error_passkey_session_refresh_failed";
    }
    if raw == "error_unsupported_target" || lower.contains("requires hydrate") {
        return "error_unsupported_target";
    }

    "error_internal"
}

#[component]
pub fn LoginPage() -> impl IntoView {
    let i18n = use_i18n();
    let login_action = ServerAction::<LoginUser>::new();
    let auth = expect_context::<AuthState>();
    let navigate = use_navigate();
    let navigate_for_effect = navigate.clone();

    let csrf_sig = use_context::<crate::csrf::CsrfContext>()
        .map(|c| c.0)
        .unwrap_or_else(|| RwSignal::new(None::<String>));

    let csrf_value = move || csrf_sig.read().clone().unwrap_or_default();
    let csrf_ready = move || csrf_sig.read().is_some();

    let csrf_refresh = expect_context::<RwSignal<()>>();
    let protected_path = move || {
        let locale = i18n.get_locale_untracked();
        lp(
            locale,
            td_string!(locale, routes.protected_path),
        )
    };
    let account_password_path = move || {
        let locale = i18n.get_locale_untracked();
        lp(
            locale,
            td_string!(locale, routes.account_password_path),
        )
    };

    let passkey_pending = RwSignal::new(false);
    let passkey_error = RwSignal::new(None::<String>);

    let localize_error = {
        let i18n = i18n.clone();
        move |raw: String| -> String {
            let locale = i18n.get_locale();
            match normalize_login_error_key(&raw) {
                "error_invalid_credentials" => {
                    td_string!(locale, login.error_invalid_credentials).to_string()
                }
                "error_missing_csrf" => td_string!(locale, login.error_missing_csrf).to_string(),
                "error_security_token" => td_string!(locale, login.error_security_token).to_string(),
                "error_session" => td_string!(locale, login.error_session).to_string(),
                "error_session_refresh_failed" => {
                    td_string!(locale, login.error_session_refresh_failed).to_string()
                }
                "error_passkey_failed" => td_string!(locale, login.error_passkey_failed).to_string(),
                "error_passkey_session_refresh_failed" => {
                    td_string!(locale, login.error_passkey_session_refresh_failed).to_string()
                }
                "error_unsupported_target" => {
                    td_string!(locale, login.error_unsupported_target).to_string()
                }
                _ => td_string!(locale, login.error_internal).to_string(),
            }
        }
    };

    Effect::new(move |_| {
        if let Some(Ok(())) = login_action.value().get() {
            auth.set_ready.set(false);
            csrf_refresh.set(());

            let navigate = navigate_for_effect.clone();
            let protected_target = protected_path();
            let account_password_target = account_password_path();
            leptos::task::spawn_local({
                let auth = auth;
                async move {
                    if let Ok(snap) = crate::auth_state::auth_snapshot().await {
                        let requires_reset = snap
                            .user
                            .as_ref()
                            .map(|u| u.password_reset_required)
                            .unwrap_or(false);

                        auth.set_user.set(snap.user);
                        auth.set_permissions.set(snap.permissions);
                        auth.set_ready.set(true);

                        let target = if requires_reset {
                            account_password_target.clone()
                        } else {
                            protected_target.clone()
                        };
                        navigate(&target, Default::default());
                    }
                }
            });
        }
    });

    let on_passkey_login = {
        let navigate = navigate.clone();
        move |_| {
            if passkey_pending.get_untracked() || !csrf_ready() {
                return;
            }

            let csrf = csrf_sig.get_untracked().unwrap_or_default();
            passkey_pending.set(true);
            passkey_error.set(None);

            let navigate = navigate.clone();
            let protected_target = protected_path();
            let account_password_target = account_password_path();
            leptos::task::spawn_local({
                let auth = auth;
                async move {
                    let result = async {
                        let rcr = webauthn_login_start().await?;
                        let pkc = get_credential(rcr).await?;
                        webauthn_login_finish(csrf, pkc).await?;
                        Ok::<(), ServerFnError>(())
                    }
                    .await;

                    match result {
                        Ok(()) => {
                            auth.set_ready.set(false);
                            csrf_refresh.set(());

                            match crate::auth_state::auth_snapshot().await {
                                Ok(snap) => {
                                    let requires_reset = snap
                                        .user
                                        .as_ref()
                                        .map(|u| u.password_reset_required)
                                        .unwrap_or(false);

                                    auth.set_user.set(snap.user);
                                    auth.set_permissions.set(snap.permissions);
                                    auth.set_ready.set(true);

                                    let target = if requires_reset {
                                        account_password_target.clone()
                                    } else {
                                        protected_target.clone()
                                    };
                                    navigate(&target, Default::default());
                                }
                                Err(_) => {
                                    auth.set_ready.set(true);
                                    passkey_error.set(Some(
                                        "error_passkey_session_refresh_failed".to_string(),
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            passkey_error.set(Some(e.to_string()));
                        }
                    }

                    passkey_pending.set(false);
                }
            });
        }
    };

    view! {
        <h1>{t!(i18n, login.title)}</h1>

        <Show when=move || auth.ready.get() && auth.requires_password_reset()>
            <div class="error" style="margin-bottom: 0.5rem;">
                {t!(i18n, login.error_password_reset_required)}
            </div>
            <A
                href=move || account_password_path()
                attr:style="display:inline-block; margin-bottom: 0.75rem;"
            >
                {t!(i18n, account.submit_change_password)}
            </A>
        </Show>

        {move || login_action.value().get()
            .and_then(|res| res.err())
            .map(|err| view! { <div class="error">{localize_error(err.to_string())}</div> })
        }

        {move || passkey_error.get()
            .map(|err| view! { <div class="error">{localize_error(err)}</div> })
        }

        <ActionForm action=login_action>
            <input type="hidden" name="csrf" value=csrf_value />

            <label>
                {t!(i18n, login.username)}
                <input
                    type="text"
                    name="username"
                    placeholder=move || td_string!(i18n.get_locale(), login.username).to_string()

                />
            </label>

            <label>
                {t!(i18n, login.password)}
                <input
                    type="password"
                    name="password"
                    placeholder=move || td_string!(i18n.get_locale(), login.password).to_string()

                />
            </label>

            <button
                type="submit"
                disabled=move || !csrf_ready() || login_action.pending().get() || passkey_pending.get()
            >
                {move || {
                    if login_action.pending().get() {
                        t!(i18n, login.button_pending)()
                    } else {
                        t!(i18n, login.button)()
                    }
                }}
            </button>
        </ActionForm>

        <button
            type="button"
            on:click=on_passkey_login
            disabled=move || !csrf_ready() || login_action.pending().get() || passkey_pending.get()
            style="margin-top: 0.75rem;"
        >
            {move || {
                if passkey_pending.get() {
                    t!(i18n, login.passkey_button_pending)()
                } else {
                    t!(i18n, login.passkey_button)()
                }
            }}
        </button>
    }
}
