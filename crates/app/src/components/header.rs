use crate::auth_state::AuthState;
use crate::components::lang_switch::LangSwitch;
use crate::i18n::*;
use crate::i18n_utils::lp;
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn Header() -> impl IntoView {
    let i18n = use_i18n();
    let auth = expect_context::<AuthState>();
    let locale_now = Signal::derive(move || i18n.get_locale());

    view! {
        <header>
            <nav>
                <A href=move || lp(locale_now.get(), td_string!(locale_now.get(), routes.home_path))>
                    {t!(i18n, header.home)}
                </A>
                " | "
                <A href=move || lp(locale_now.get(), td_string!(locale_now.get(), routes.contact_path))>
                    {t!(i18n, header.contact)}
                </A>
                " | "

                <Show when=move || auth.ready.get() fallback=|| ()>
                    <Show when=move || !auth.logged_in() fallback=|| ()>
                        <A href=move || lp(locale_now.get(), td_string!(locale_now.get(), routes.login_path))>
                            {t!(i18n, header.login)}
                        </A>
                        " | "
                    </Show>

                    <Show when=move || auth.logged_in() fallback=|| ()>
                        <A href=move || lp(locale_now.get(), td_string!(locale_now.get(), routes.protected_path))>
                            {t!(i18n, header.protected)}
                        </A>
                        " | "
                        <A href=move || lp(locale_now.get(), td_string!(locale_now.get(), routes.account_path))>{t!(i18n, header.account)}</A>
                        " | "
                        <Show when=move || auth.is_admin() fallback=|| ()>
                            <A href=move || lp(locale_now.get(), td_string!(locale_now.get(), routes.admin_path))>
                                {t!(i18n, header.admin)}
                            </A>
                            " | "
                        </Show>
                        <A href=move || lp(locale_now.get(), td_string!(locale_now.get(), routes.logout_path))>
                            {t!(i18n, header.logout)}
                        </A>
                        " | "
                    </Show>
                </Show>

                <LangSwitch />
            </nav>
            <hr/>
        </header>
    }
}
