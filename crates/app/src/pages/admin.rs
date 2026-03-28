use leptos::prelude::*;
use crate::contexts::AuthState;
use crate::contexts::CsrfContext;
use crate::i18n::*;

use crate::features::admin::users_table::UsersTable;

#[component]
pub fn AdminPage() -> impl IntoView {
    let i18n = use_i18n();
    let auth = expect_context::<AuthState>();

    let csrf_sig = use_context::<CsrfContext>()
        .map(|c| c.0)
        .unwrap_or_else(|| RwSignal::new(None::<String>));

    // (is_success, message_key_or_raw)
    let flash = RwSignal::<Option<(bool, String)>>::new(None);

    let render_flash = Callback::new({
        let i18n = i18n.clone();
        move |raw: String| -> String {
            let locale = i18n.get_locale_untracked();
            match raw.as_str() {
                // success/info
                "msg_missing_csrf" => td_string!(locale, admin.msg_missing_csrf).to_string(),
                "msg_user_created" => td_string!(locale, admin.msg_user_created).to_string(),
                "msg_user_updated" => td_string!(locale, admin.msg_user_updated).to_string(),
                "msg_user_deleted" => td_string!(locale, admin.msg_user_deleted).to_string(),
                "msg_user_status_updated" => td_string!(locale, admin.msg_user_status_updated).to_string(),
                "msg_password_reset_flag_set" => td_string!(locale, admin.msg_password_reset_flag_set).to_string(),

                // errors
                "err_not_authenticated" => td_string!(locale, admin.err_not_authenticated).to_string(),
                "err_forbidden" => td_string!(locale, admin.err_forbidden).to_string(),
                "err_server_only" => td_string!(locale, admin.err_server_only).to_string(),
                "err_username_required" => td_string!(locale, admin.err_username_required).to_string(),
                "err_password_too_short" => td_string!(locale, admin.err_password_too_short).to_string(),
                "err_username_exists" => td_string!(locale, admin.err_username_exists).to_string(),
                "err_user_not_found" => td_string!(locale, admin.err_user_not_found).to_string(),
                "err_invalid_email" => td_string!(locale, admin.err_invalid_email).to_string(),
                "err_email_taken" => td_string!(locale, admin.err_email_taken).to_string(),
                "err_invalid_status" => td_string!(locale, admin.err_invalid_status).to_string(),
                "err_invalid_role" => td_string!(locale, admin.err_invalid_role).to_string(),
                "err_roles_required" => td_string!(locale, admin.err_roles_required).to_string(),
                "err_cannot_delete_self" => td_string!(locale, admin.err_cannot_delete_self).to_string(),
                _ => raw,
            }
        }
    });

    view! {
        <section>
            <h1>{t!(i18n, admin.title)}</h1>

            <p>{t!(i18n, admin.welcome)} " " {move || auth.username().unwrap_or_default()}</p>
            <p>{t!(i18n, admin.fake_note)}</p>

            <Show
                when=move || auth.ready.get()
                fallback=move || view! { <p>{t!(i18n, admin.working)}</p> }
            >
                <Show
                    when=move || auth.is_admin()
                    fallback=move || view! { <p style="color:red;">{t!(i18n, admin.forbidden)}</p> }
                >
                    <Show when=move || flash.get().is_some() fallback=|| ()>
                        <p
                            role="status"
                            aria-live="polite"
                            style=move || {
                                match flash.get() {
                                    Some((ok, _)) => {
                                        if ok { "color: green;" } else { "color: red;" }
                                    }
                                    None => "",
                                }
                            }
                        >
                            {move || {
                                flash
                                    .get()
                                    .map(|(_, msg)| render_flash.run(msg))
                                    .unwrap_or_default()
                            }}
                        </p>
                    </Show>

                    <UsersTable
                        csrf_sig=csrf_sig
                        flash=flash
                        on_changed=Callback::new(move |_| {})
                    />
                </Show>
            </Show>
        </section>
    }
}
