use crate::components::lang_switch::LangSwitch;
use crate::i18n::*;
use leptos::prelude::*;

#[component]
pub fn Header() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <header>
            <nav>
                <a href="/">{t!(i18n, header.home)}</a> |
                <a href="/kontakt">{t!(i18n, header.contact)}</a> |
                 <a href="/login">{"Login"}</a> |
                 <a href="/protected">{"Protected"}</a> |
                <LangSwitch />
            </nav>
            <hr/>
        </header>
    }
}
