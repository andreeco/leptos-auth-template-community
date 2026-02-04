use crate::i18n::*;
use leptos::prelude::*;

#[component]
pub fn Footer() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <footer style="margin-top:2em;">
            <hr/>
            <p>
                {t!(i18n, footer.copyright)}
                <a href="/impressum">{t!(i18n, footer.imprint)}</a>
                " | "
                <a href="/datenschutzerklaerung">{t!(i18n, footer.privacy)}</a>
            </p>
        </footer>
    }
}
