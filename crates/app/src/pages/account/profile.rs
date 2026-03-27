use crate::features::account::{account_profile_get, account_profile_update};
use crate::contexts::AuthState;
use crate::contexts::CsrfContext;
use crate::i18n::*;
use crate::i18n_utils::localized_path;
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;

#[component]
pub fn AccountProfilePage() -> impl IntoView {
    let i18n = use_i18n();
    let auth = expect_context::<AuthState>();

    let csrf_sig = use_context::<CsrfContext>()
        .map(|c| c.0)
        .unwrap_or_else(|| RwSignal::new(None::<String>));
    let csrf_ready = move || csrf_sig.read().is_some();
    let csrf_refresh = use_context::<RwSignal<()>>().unwrap_or_else(|| RwSignal::new(()));

    let href_login =
        move || localized_path(i18n.get_locale(), td_string!(i18n.get_locale(), routes.login_path));
    let href_account = move || {
        localized_path(
            i18n.get_locale(),
            td_string!(i18n.get_locale(), routes.account_path),
        )
    };

    let username = RwSignal::new(String::new());
    let first_name = RwSignal::new(String::new());
    let last_name = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());

    let pending = RwSignal::new(false);
    let loaded = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let success = RwSignal::new(None::<String>);

    let map_server_error = {
        let i18n = i18n.clone();
        move |raw: String| -> String {
            match raw.as_str() {
                "err_missing_csrf" => {
                    td_string!(i18n.get_locale(), account.error_missing_csrf).to_string()
                }
                "err_user_not_found" => {
                    td_string!(i18n.get_locale(), account.error_user_not_found).to_string()
                }
                "err_not_authenticated" => {
                    td_string!(i18n.get_locale(), account.error_not_authenticated).to_string()
                }
                "err_forbidden" => td_string!(i18n.get_locale(), account.error_forbidden).to_string(),
                "err_session_error" => td_string!(i18n.get_locale(), account.error_session).to_string(),
                "err_invalid_email" => {
                    td_string!(i18n.get_locale(), account.error_invalid_email).to_string()
                }
                "err_email_taken" => td_string!(i18n.get_locale(), account.error_email_taken).to_string(),
                "err_username_exists" => {
                    td_string!(i18n.get_locale(), account.error_username_exists).to_string()
                }
                _ => td_string!(i18n.get_locale(), account.error_unknown).to_string(),
            }
        }
    };

    let load_profile = Callback::new(move |_| {
        if pending.get_untracked() {
            return;
        }

        pending.set(true);
        error.set(None);

        spawn_local(async move {
            match account_profile_get().await {
                Ok(profile) => {
                    username.set(profile.username);
                    first_name.set(profile.first_name);
                    last_name.set(profile.last_name);
                    email.set(profile.email);
                    loaded.set(true);
                }
                Err(e) => {
                    error.set(Some(map_server_error(e.to_string())));
                }
            }

            pending.set(false);
        });
    });

    Effect::new({
        let load_profile = load_profile.clone();
        move |_| {
            if auth.ready.get() && auth.logged_in() && !loaded.get() && !pending.get() {
                username.set(auth.username().unwrap_or_default());
                load_profile.run(());
            }
        }
    });

    let on_submit = {
        let i18n = i18n.clone();
        move |ev: SubmitEvent| {
            ev.prevent_default();

            if pending.get_untracked() {
                return;
            }

            let Some(csrf) = csrf_sig.get_untracked() else {
                error.set(Some(
                    td_string!(i18n.get_locale(), account.error_missing_csrf).to_string(),
                ));
                success.set(None);
                return;
            };

            let uname = username.get_untracked();
            let first = first_name.get_untracked();
            let last = last_name.get_untracked();
            let mail = email.get_untracked();

            pending.set(true);
            error.set(None);
            success.set(None);

            spawn_local(async move {
                match account_profile_update(csrf, uname, first, last, mail).await {
                    Ok(updated) => {
                        first_name.set(updated.first_name);
                        last_name.set(updated.last_name);
                        email.set(updated.email);

                        success.set(Some(
                            td_string!(i18n.get_locale(), account.success_profile_updated).to_string(),
                        ));
                        error.set(None);
                        loaded.set(true);

                        // Server rotates token on successful updates.
                        csrf_refresh.set(());

                        // Refresh auth snapshot so header/account state stays in sync.
                        auth.set_ready.set(false);
                        match crate::contexts::auth_snapshot().await {
                            Ok(snap) => {
                                auth.set_user.set(snap.user);
                                auth.set_permissions.set(snap.permissions);
                                username.set(auth.username().unwrap_or_default());
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
            <h1>{t!(i18n, account.profile_title)}</h1>

            <Show
                when=move || auth.ready.get()
                fallback=move || view! { <p>{t!(i18n, account.checking_account_status)}</p> }
            >
                <Show
                    when=move || auth.logged_in()
                    fallback=move || view! {
                        <p>{t!(i18n, account.login_required_manage_account)}</p>
                        <p><A href=href_login>{t!(i18n, account.go_to_login)}</A></p>
                    }
                >
                    <p>
                        {t!(i18n, account.signed_in_as)}
                        " "
                        <strong>
                            {move || auth
                                .username()
                                .unwrap_or_else(|| td_string!(i18n.get_locale(), account.fallback_username).to_string())}
                        </strong>
                    </p>

                    <Show
                        when=move || loaded.get() || pending.get()
                        fallback=move || view! { <p>{t!(i18n, account.profile_loading)}</p> }
                    >
                        <form on:submit=on_submit>
                            <div>
                                <label for="profile_username">{t!(i18n, login.username)}</label><br/>
                                <input
                                    id="profile_username"
                                    type="text"
                                    prop:value=move || username.get()
                                    on:input:target=move |ev| username.set(ev.target().value())
                                    disabled=move || pending.get()
                                />
                            </div>

                            <div style="margin-top: 0.75rem;">
                                <label for="profile_first_name">{t!(i18n, account.profile_first_name_label)}</label><br/>
                                <input
                                    id="profile_first_name"
                                    type="text"
                                    prop:value=move || first_name.get()
                                    on:input:target=move |ev| first_name.set(ev.target().value())
                                    disabled=move || pending.get()
                                />
                            </div>

                            <div style="margin-top: 0.75rem;">
                                <label for="profile_last_name">{t!(i18n, account.profile_last_name_label)}</label><br/>
                                <input
                                    id="profile_last_name"
                                    type="text"
                                    prop:value=move || last_name.get()
                                    on:input:target=move |ev| last_name.set(ev.target().value())
                                    disabled=move || pending.get()
                                />
                            </div>

                            <div style="margin-top: 0.75rem;">
                                <label for="profile_email">{t!(i18n, account.profile_email_label)}</label><br/>
                                <input
                                    id="profile_email"
                                    type="email"
                                    autocomplete="email"
                                    prop:value=move || email.get()
                                    on:input:target=move |ev| email.set(ev.target().value())
                                    disabled=move || pending.get()
                                />
                            </div>

                            <div style="margin-top: 1rem;">
                                <button type="submit" disabled=move || !csrf_ready() || pending.get()>
                                    {move || {
                                        if pending.get() {
                                            td_string!(i18n.get_locale(), account.submit_saving_profile).to_string()
                                        } else {
                                            td_string!(i18n.get_locale(), account.profile_save).to_string()
                                        }
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
                    </Show>

                    <p style="margin-top: 1rem;">
                        <A href=href_account>{t!(i18n, account.back_to_account)}</A>
                    </p>
                </Show>
            </Show>
        </section>
    }
}
