use crate::features::account::account_change_password;
use crate::contexts::AuthState;
use crate::contexts::CsrfContext;
use crate::i18n::*;
use crate::i18n_utils::localized_path;
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;

#[component]
pub fn AccountPasswordPage() -> impl IntoView {
    let i18n = use_i18n();
    let auth = expect_context::<AuthState>();

    let current_password = RwSignal::new(String::new());
    let new_password = RwSignal::new(String::new());
    let confirm_password = RwSignal::new(String::new());

    let pending = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let success = RwSignal::new(None::<String>);

    let csrf_sig = use_context::<CsrfContext>()
        .map(|c| c.0)
        .unwrap_or_else(|| RwSignal::new(None::<String>));
    let csrf_ready = move || csrf_sig.read().is_some();

    let csrf_refresh = use_context::<RwSignal<()>>().unwrap_or_else(|| RwSignal::new(()));

    let href_login =
        move || localized_path(i18n.get_locale(), td_string!(i18n.get_locale(), routes.login_path));
    let href_account =
        move || localized_path(i18n.get_locale(), td_string!(i18n.get_locale(), routes.account_path));

    let map_server_error = {
        let i18n = i18n.clone();
        move |raw: String| -> String {
            match raw.as_str() {
                "err_missing_csrf" => {
                    td_string!(i18n.get_locale(), account.error_missing_csrf).to_string()
                }
                "err_password_mismatch" => {
                    td_string!(i18n.get_locale(), account.error_password_mismatch).to_string()
                }
                "err_password_too_short" => {
                    td_string!(i18n.get_locale(), account.error_password_too_short).to_string()
                }
                "err_current_password_incorrect" => {
                    td_string!(i18n.get_locale(), account.error_current_password_incorrect)
                        .to_string()
                }
                "err_user_not_found" => {
                    td_string!(i18n.get_locale(), account.error_user_not_found).to_string()
                }
                "err_not_authenticated" => {
                    td_string!(i18n.get_locale(), account.error_not_authenticated).to_string()
                }
                "err_forbidden" => {
                    td_string!(i18n.get_locale(), account.error_forbidden).to_string()
                }
                "err_session_error" => {
                    td_string!(i18n.get_locale(), account.error_session).to_string()
                }
                _ => td_string!(i18n.get_locale(), account.error_unknown).to_string(),
            }
        }
    };

    let on_submit = {
        let i18n = i18n.clone();

        move |ev: SubmitEvent| {
            ev.prevent_default();

            if pending.get_untracked() {
                return;
            }

            let csrf = match csrf_sig.get_untracked() {
                Some(token) => token,
                None => {
                    error.set(Some(
                        td_string!(i18n.get_locale(), account.error_missing_csrf).to_string(),
                    ));
                    success.set(None);
                    return;
                }
            };

            let current = current_password.get_untracked();
            let new_pw = new_password.get_untracked();
            let confirm = confirm_password.get_untracked();

            if new_pw != confirm {
                error.set(Some(
                    td_string!(i18n.get_locale(), account.error_password_mismatch).to_string(),
                ));
                success.set(None);
                return;
            }

            if new_pw.len() < 10 {
                error.set(Some(
                    td_string!(i18n.get_locale(), account.error_password_too_short).to_string(),
                ));
                success.set(None);
                return;
            }

            pending.set(true);
            error.set(None);
            success.set(None);

            spawn_local(async move {
                match account_change_password(csrf, current, new_pw, confirm).await {
                    Ok(()) => {
                        current_password.set(String::new());
                        new_password.set(String::new());
                        confirm_password.set(String::new());
                        success.set(Some(
                            td_string!(i18n.get_locale(), account.success_password_updated)
                                .to_string(),
                        ));
                        error.set(None);

                        // Server rotates token on success; refresh client copy.
                        csrf_refresh.set(());

                        // Refresh auth snapshot so password-reset-required state clears immediately.
                        auth.set_ready.set(false);
                        match crate::contexts::auth_snapshot().await {
                            Ok(snap) => {
                                auth.set_user.set(snap.user);
                                auth.set_permissions.set(snap.permissions);
                            }
                            Err(_) => {}
                        }
                        auth.set_ready.set(true);
                    }
                    Err(e) => {
                        error.set(Some(map_server_error(e.to_string())));
                        success.set(None);
                    }
                }

                pending.set(false);
            });
        }
    };

    view! {
        <section>
            <h1>{t!(i18n, account.password_title)}</h1>

            <Show
                when=move || auth.ready.get()
                fallback=move || view! { <p>{t!(i18n, account.checking_account_status)}</p> }
            >
                <Show
                    when=move || auth.logged_in()
                    fallback=move || view! {
                        <p>{t!(i18n, account.login_required_change_password)}</p>
                        <p><A href=href_login>{t!(i18n, account.go_to_login)}</A></p>
                    }
                >
                    <p>
                        {t!(i18n, account.signed_in_as)}
                        " "
                        <strong>{move || auth.username().unwrap_or_else(|| td_string!(i18n.get_locale(), account.fallback_username).to_string())}</strong>
                    </p>

                    <h2>{t!(i18n, account.security)}</h2>

                    <form on:submit=on_submit>
                        <div>
                            <label for="current_password">{t!(i18n, account.current_password_label)}</label><br/>
                            <input
                                id="current_password"
                                type="password"
                                autocomplete="current-password"
                                prop:value=move || current_password.get()
                                on:input:target=move |ev| current_password.set(ev.target().value())
                                disabled=move || pending.get()
                            />
                        </div>

                        <div style="margin-top: 0.75rem;">
                            <label for="new_password">{t!(i18n, account.new_password_label)}</label><br/>
                            <input
                                id="new_password"
                                type="password"
                                autocomplete="new-password"
                                prop:value=move || new_password.get()
                                on:input:target=move |ev| new_password.set(ev.target().value())
                                disabled=move || pending.get()
                            />
                        </div>

                        <div style="margin-top: 0.75rem;">
                            <label for="confirm_password">{t!(i18n, account.confirm_new_password_label)}</label><br/>
                            <input
                                id="confirm_password"
                                type="password"
                                autocomplete="new-password"
                                prop:value=move || confirm_password.get()
                                on:input:target=move |ev| confirm_password.set(ev.target().value())
                                disabled=move || pending.get()
                            />
                        </div>

                        <div style="margin-top: 1rem;">
                            <button
                                type="submit"
                                disabled=move || !csrf_ready() || pending.get()
                            >
                                {move || if pending.get() {
                                    td_string!(i18n.get_locale(), account.submit_updating).to_string()
                                } else {
                                    td_string!(i18n.get_locale(), account.submit_change_password).to_string()
                                }}
                            </button>
                        </div>
                    </form>

                    <Show when=move || success.get().is_some()>
                        <p style="color: green; margin-top: 0.75rem;">
                            {move || success.get().unwrap_or_default()}
                        </p>
                    </Show>

                    <Show when=move || error.get().is_some()>
                        <p style="color: red; margin-top: 0.75rem;">
                            {move || error.get().unwrap_or_default()}
                        </p>
                    </Show>

                    <p style="margin-top: 1rem;">
                        <A href=href_account>{t!(i18n, account.back_to_account)}</A>
                    </p>
                </Show>
            </Show>
        </section>
    }
}
