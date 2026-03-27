use crate::account::{
    account_webauthn_delete, account_webauthn_list, account_webauthn_register_finish,
    account_webauthn_register_start, create_credential, WebauthnCredentialRow,
};
use crate::auth_state::AuthState;
use crate::csrf::CsrfContext;
use crate::i18n::*;
use crate::i18n_paths::lp;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;

fn localize_webauthn_error(locale: Locale, raw: &str) -> String {
    match raw {
        "err_not_authenticated" => {
            td_string!(locale, account_webauthn.err_not_authenticated).to_string()
        }
        "err_forbidden" => td_string!(locale, account_webauthn.err_forbidden).to_string(),
        "err_user_not_found" => td_string!(locale, account_webauthn.err_user_not_found).to_string(),
        "err_session_error" => td_string!(locale, account_webauthn.err_session_error).to_string(),
        "err_missing_registration_state" => {
            td_string!(locale, account_webauthn.err_missing_registration_state).to_string()
        }
        "err_missing_discoverable_auth_state" => {
            td_string!(locale, account_webauthn.err_missing_discoverable_auth_state).to_string()
        }
        "err_unknown_user_for_discoverable_credential" => {
            td_string!(locale, account_webauthn.err_unknown_user_for_discoverable_credential).to_string()
        }
        "err_no_passkeys_registered_for_user" => {
            td_string!(locale, account_webauthn.err_no_passkeys_registered_for_user).to_string()
        }
        "err_passkey_json_serialize_failed" => {
            td_string!(locale, account_webauthn.err_passkey_json_serialize_failed).to_string()
        }
        "err_invalid_stored_passkey_json" => {
            td_string!(locale, account_webauthn.err_invalid_stored_passkey_json).to_string()
        }
        "err_base64url_decode_failed" => {
            td_string!(locale, account_webauthn.err_base64url_decode_failed).to_string()
        }
        "err_missing_webauthn_user_handle" => {
            td_string!(locale, account_webauthn.err_missing_webauthn_user_handle).to_string()
        }
        "err_invalid_webauthn_user_handle" => {
            td_string!(locale, account_webauthn.err_invalid_webauthn_user_handle).to_string()
        }
        "err_passkey_name_required" => {
            td_string!(locale, account_webauthn.msg_passkey_name_required).to_string()
        }
        "err_webauthn_requires_wasm" => {
            td_string!(locale, account_webauthn.err_webauthn_requires_wasm).to_string()
        }
        "err_no_window" => td_string!(locale, account_webauthn.err_no_window).to_string(),
        "err_navigator_credentials_create_failed" => {
            td_string!(locale, account_webauthn.err_navigator_credentials_create_failed).to_string()
        }
        "err_create_credential_rejected" => {
            td_string!(locale, account_webauthn.err_create_credential_rejected).to_string()
        }
        "err_create_credential_invalid_response" => {
            td_string!(locale, account_webauthn.err_create_credential_invalid_response).to_string()
        }
        "err_navigator_credentials_get_failed" => {
            td_string!(locale, account_webauthn.err_navigator_credentials_get_failed).to_string()
        }
        "err_get_credential_rejected" => {
            td_string!(locale, account_webauthn.err_get_credential_rejected).to_string()
        }
        "err_get_credential_invalid_response" => {
            td_string!(locale, account_webauthn.err_get_credential_invalid_response).to_string()
        }
        "err_server_only" => td_string!(locale, account_webauthn.err_server_only).to_string(),
        _ => raw.to_string(),
    }
}

#[component]
pub fn AccountWebauthnPage() -> impl IntoView {
    let i18n = use_i18n();
    let auth = expect_context::<AuthState>();

    let csrf_sig = use_context::<CsrfContext>()
        .map(|c| c.0)
        .unwrap_or_else(|| RwSignal::new(None::<String>));
    let csrf_ready = move || csrf_sig.read().is_some();

    let csrf_refresh = use_context::<RwSignal<()>>().unwrap_or_else(|| RwSignal::new(()));

    let pending_register = RwSignal::new(false);
    let pending_delete_id = RwSignal::new(None::<i64>);
    let message = RwSignal::new(None::<String>);
    let draft_name = RwSignal::new(String::new());

    let list_refresh = RwSignal::new(0_u64);

    let credentials_res = Resource::new(
        move || (list_refresh.get(), auth.ready.get(), auth.logged_in()),
        move |(_, ready, logged_in)| async move {
            if ready && logged_in {
                account_webauthn_list().await
            } else {
                Ok::<Vec<WebauthnCredentialRow>, ServerFnError>(Vec::new())
            }
        },
    );

    let register_passkey = {
        let i18n = i18n.clone();
        move |_| {
            if pending_register.get_untracked() || pending_delete_id.get_untracked().is_some() {
                return;
            }

            let Some(csrf) = csrf_sig.get_untracked() else {
                message.set(Some(
                    td_string!(i18n.get_locale(), account_webauthn.msg_missing_csrf).to_string(),
                ));
                return;
            };

            let name = draft_name.get_untracked().trim().to_string();
            if name.is_empty() {
                message.set(Some(
                    td_string!(i18n.get_locale(), account_webauthn.msg_passkey_name_required)
                        .to_string(),
                ));
                return;
            }

            pending_register.set(true);
            message.set(None);

            let locale = i18n.get_locale();
            spawn_local(async move {
                let result = async {
                    let ccr = account_webauthn_register_start(name).await?;
                    let rpkc = create_credential(ccr).await?;
                    account_webauthn_register_finish(csrf, rpkc).await?;
                    Ok::<(), ServerFnError>(())
                }
                .await;

                match result {
                    Ok(()) => {
                        draft_name.set(String::new());
                        message.set(Some(
                            td_string!(locale, account_webauthn.msg_passkey_registered).to_string(),
                        ));
                        csrf_refresh.set(());
                        list_refresh.update(|v| *v += 1);
                    }
                    Err(e) => {
                        let detail = localize_webauthn_error(locale, &e.to_string());
                        message.set(Some(format!(
                            "{}: {detail}",
                            td_string!(locale, account_webauthn.err_passkey_registration_failed_prefix)
                        )));
                    }
                }

                pending_register.set(false);
            });
        }
    };

    let delete_passkey = {
        let i18n = i18n.clone();
        move |id: i64| {
            if pending_register.get_untracked() || pending_delete_id.get_untracked().is_some() {
                return;
            }

            let Some(csrf) = csrf_sig.get_untracked() else {
                message.set(Some(
                    td_string!(i18n.get_locale(), account_webauthn.msg_missing_csrf).to_string(),
                ));
                return;
            };

            pending_delete_id.set(Some(id));
            message.set(None);

            let locale = i18n.get_locale();
            spawn_local(async move {
                match account_webauthn_delete(csrf, id).await {
                    Ok(()) => {
                        message.set(Some(
                            td_string!(locale, account_webauthn.msg_passkey_deleted).to_string(),
                        ));
                        list_refresh.update(|v| *v += 1);
                    }
                    Err(e) => {
                        let detail = localize_webauthn_error(locale, &e.to_string());
                        message.set(Some(format!(
                            "{}: {detail}",
                            td_string!(locale, account_webauthn.err_delete_failed_prefix)
                        )));
                    }
                }

                pending_delete_id.set(None);
            });
        }
    };

    view! {
        <section>
            <h1>{t!(i18n, account_webauthn.title)}</h1>

            <Show
                when=move || auth.ready.get()
                fallback=move || view! { <p>{t!(i18n, account_webauthn.checking_account_status)}</p> }
            >
                <Show
                    when=move || auth.logged_in()
                    fallback=move || view! {
                        <p>{t!(i18n, account_webauthn.need_logged_in_manage_passkeys)}</p>
                        <p>
                            <A href=move || lp(i18n.get_locale(), td_string!(i18n.get_locale(), routes.login_path))>
                                {t!(i18n, account_webauthn.go_to_login)}
                            </A>
                        </p>
                    }
                >
                    <p>
                        {t!(i18n, account_webauthn.signed_in_as)}
                        " "
                        <strong>
                            {move || auth
                                .username()
                                .unwrap_or_else(|| td_string!(i18n.get_locale(), account_webauthn.default_username).to_string())}
                        </strong>
                    </p>

                    <section style="margin-top: 1rem;">
                        <p>{t!(i18n, account_webauthn.enroll_help)}</p>
                        <p style="color: #a35a00;">
                            <strong>{t!(i18n, account_webauthn.origin_warning_title)}</strong>
                            " "
                            {t!(i18n, account_webauthn.origin_warning_body)}
                        </p>
                    </section>

                    <hr style="margin: 1rem 0;"/>

                    <section>
                        <h2>{t!(i18n, account_webauthn.your_passkeys)}</h2>

                        <Transition fallback=move || view! { <p>{t!(i18n, account_webauthn.loading_passkeys)}</p> }>
                            {move || {
                                match credentials_res.get() {
                                    None => view! { <p>{t!(i18n, account_webauthn.loading_passkeys)}</p> }.into_any(),
                                    Some(Err(e)) => {
                                        let detail = localize_webauthn_error(i18n.get_locale(), &e.to_string());
                                        view! {
                                            <p style="color: red;">
                                                {format!("{}: {detail}", td_string!(i18n.get_locale(), account_webauthn.err_failed_load_passkeys_prefix))}
                                            </p>
                                        }.into_any()
                                    }
                                    Some(Ok(rows)) if rows.is_empty() => {
                                        view! {
                                            <table style="margin: 0 auto;">
                                                <thead>
                                                    <tr>
                                                        <th>{t!(i18n, account_webauthn.passkey_name_label)}</th>
                                                        <th>{t!(i18n, account_webauthn.created_at_prefix)}</th>
                                                        <th>"State"</th>
                                                        <th>"Actions"</th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    <tr style="background: rgba(0,0,0,0.03);">
                                                        <td>
                                                            <input
                                                                id="passkey_name"
                                                                type="text"
                                                                bind:value=draft_name
                                                                placeholder=move || td_string!(i18n.get_locale(), account_webauthn.passkey_name_placeholder).to_string()
                                                                disabled=move || pending_register.get() || pending_delete_id.get().is_some()
                                                            />
                                                        </td>
                                                        <td>"—"</td>
                                                        <td>
                                                            {move || {
                                                                if pending_register.get() {
                                                                    td_string!(i18n.get_locale(), account_webauthn.registering).to_string()
                                                                } else {
                                                                    "ready".to_string()
                                                                }
                                                            }}
                                                        </td>
                                                        <td style="display:flex; gap:0.4rem; justify-content:center;">
                                                            <button
                                                                type="button"
                                                                on:click=register_passkey
                                                                disabled=move || !csrf_ready() || pending_register.get() || pending_delete_id.get().is_some()
                                                            >
                                                                {t!(i18n, account_webauthn.register_passkey)}
                                                            </button>
                                                        </td>
                                                    </tr>
                                                    <tr>
                                                        <td colspan="4">{t!(i18n, account_webauthn.no_passkeys_registered)}</td>
                                                    </tr>
                                                </tbody>
                                            </table>
                                        }.into_any()
                                    }
                                    Some(Ok(rows)) => {
                                        view! {
                                            <table style="margin: 0 auto;">
                                                <thead>
                                                    <tr>
                                                        <th>{t!(i18n, account_webauthn.passkey_name_label)}</th>
                                                        <th>{t!(i18n, account_webauthn.created_at_prefix)}</th>
                                                        <th>"State"</th>
                                                        <th>"Actions"</th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    <tr style="background: rgba(0,0,0,0.03);">
                                                        <td>
                                                            <input
                                                                id="passkey_name"
                                                                type="text"
                                                                bind:value=draft_name
                                                                placeholder=move || td_string!(i18n.get_locale(), account_webauthn.passkey_name_placeholder).to_string()
                                                                disabled=move || pending_register.get() || pending_delete_id.get().is_some()
                                                            />
                                                        </td>
                                                        <td>"—"</td>
                                                        <td>
                                                            {move || {
                                                                if pending_register.get() {
                                                                    td_string!(i18n.get_locale(), account_webauthn.registering).to_string()
                                                                } else {
                                                                    "ready".to_string()
                                                                }
                                                            }}
                                                        </td>
                                                        <td style="display:flex; gap:0.4rem; justify-content:center;">
                                                            <button
                                                                type="button"
                                                                on:click=register_passkey
                                                                disabled=move || !csrf_ready() || pending_register.get() || pending_delete_id.get().is_some()
                                                            >
                                                                {t!(i18n, account_webauthn.register_passkey)}
                                                            </button>
                                                        </td>
                                                    </tr>

                                                    <For each=move || rows.clone() key=|c| c.id let:cred>
                                                        {{
                                                            let cred_id = cred.id;
                                                            let cred_name = cred.name.clone();
                                                            let cred_created = cred.created_at.clone();

                                                            view! {
                                                                <tr style=move || if pending_delete_id.get() == Some(cred_id) { "opacity: 0.65;" } else { "" }>
                                                                    <td>{cred_name}</td>
                                                                    <td>
                                                                        {format!(
                                                                            "{} {}",
                                                                            td_string!(i18n.get_locale(), account_webauthn.created_at_prefix),
                                                                            cred_created
                                                                        )}
                                                                    </td>
                                                                    <td>
                                                                        {move || {
                                                                            if pending_delete_id.get() == Some(cred_id) {
                                                                                td_string!(i18n.get_locale(), account_webauthn.loading_passkeys).to_string()
                                                                            } else {
                                                                                "ready".to_string()
                                                                            }
                                                                        }}
                                                                    </td>
                                                                    <td style="text-align:center;">
                                                                        <button
                                                                            type="button"
                                                                            on:click=move |_| delete_passkey(cred_id)
                                                                            disabled=move || pending_delete_id.get().is_some() || pending_register.get()
                                                                        >
                                                                            {t!(i18n, account_webauthn.delete)}
                                                                        </button>
                                                                    </td>
                                                                </tr>
                                                            }
                                                        }}
                                                    </For>
                                                </tbody>
                                            </table>
                                        }.into_any()
                                    }
                                }
                            }}
                        </Transition>
                    </section>

                    <Show when=move || message.get().is_some()>
                        <p style="margin-top: 0.75rem;">
                            {move || message.get().unwrap_or_default()}
                        </p>
                    </Show>

                    <p style="margin-top: 1rem;">
                        <A href=move || lp(i18n.get_locale(), td_string!(i18n.get_locale(), routes.account_path))>
                            {t!(i18n, account_webauthn.back_to_account)}
                        </A>
                    </p>
                </Show>
            </Show>
        </section>
    }
}
