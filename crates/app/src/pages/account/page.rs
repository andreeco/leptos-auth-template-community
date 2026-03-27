//! Account overview page.
//!
//! This page is intentionally minimal and acts as a navigation hub for
//! account-related features (profile, password, passkeys).

use crate::contexts::AuthState;
use crate::i18n::*;
use crate::i18n_utils::localized_path;
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn AccountPage() -> impl IntoView {
    let i18n = use_i18n();
    let auth = expect_context::<AuthState>();

    let href_login =
        move || localized_path(i18n.get_locale(), td_string!(i18n.get_locale(), routes.login_path));
    let href_profile = move || {
        localized_path(
            i18n.get_locale(),
            td_string!(i18n.get_locale(), routes.account_profile_path),
        )
    };
    let href_password = move || {
        localized_path(
            i18n.get_locale(),
            td_string!(i18n.get_locale(), routes.account_password_path),
        )
    };
    let href_webauthn = move || {
        localized_path(
            i18n.get_locale(),
            td_string!(i18n.get_locale(), routes.account_webauthn_path),
        )
    };

    view! {
        <section>
            <h1>{t!(i18n, account.title)}</h1>

            <Show
                when=move || auth.ready.get()
                fallback=move || view! { <p>{t!(i18n, account.checking_account_status)}</p> }
            >
                <Show
                    when=move || auth.logged_in()
                    fallback=move || view! {
                        <p>{t!(i18n, account.login_required_manage_account)}</p>
                        <p>
                            <A href=href_login>{t!(i18n, account.go_to_login)}</A>
                        </p>
                    }
                >
                    <ul>
                        <li>
                            <A href=href_profile>{t!(i18n, account.profile_link)}</A>
                        </li>
                        <li>
                            <A href=href_password>{t!(i18n, account.change_password_link)}</A>
                        </li>
                        <li>
                            <A href=href_webauthn>{t!(i18n, account.passkeys_link)}</A>
                        </li>
                    </ul>
                </Show>
            </Show>
        </section>
    }
}
