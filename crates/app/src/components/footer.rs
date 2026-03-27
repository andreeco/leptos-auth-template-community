use crate::i18n::*;
use crate::i18n_paths::lp;
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn Footer() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <footer style="margin-top:2em;">
            <hr/>
            <p>
                {t!(i18n, footer.copyright)}
                <A href=move || lp(i18n.get_locale(), td_string!(i18n.get_locale(), routes.imprint_path))>
                    {t!(i18n, footer.imprint)}
                </A>
                " | "
                <A href=move || lp(i18n.get_locale(), td_string!(i18n.get_locale(), routes.privacy_path))>
                    {t!(i18n, footer.privacy)}
                </A>
            </p>
        </footer>
    }
}
