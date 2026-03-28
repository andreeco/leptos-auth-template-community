use crate::i18n::*;
use crate::i18n_utils::localized_path;
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn Footer() -> impl IntoView {
    let i18n = use_i18n();
    let locale_now = Signal::derive(move || i18n.get_locale());
    view! {
        <footer style="margin-top:2em;">
            <hr/>
            <p>
                {t!(i18n, footer.copyright)}
                <A href=move || localized_path(locale_now.get(), td_string!(locale_now.get(), routes.imprint_path))>
                    {t!(i18n, footer.imprint)}
                </A>
                " | "
                <A href=move || localized_path(locale_now.get(), td_string!(locale_now.get(), routes.privacy_path))>
                    {t!(i18n, footer.privacy)}
                </A>
            </p>
        </footer>
    }
}
