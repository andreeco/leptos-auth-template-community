use crate::i18n::*;
use leptos::prelude::*;
use leptos_meta::{Link as MetaLink, Title};

#[component]
pub fn Privacy() -> impl IntoView {
    let i18n = use_i18n();
    let privacy_title = move || td_string!(i18n.get_locale(), privacy.title).to_string();

    view! {
        <Title text=privacy_title />
        <MetaLink rel="alternate" hreflang="de" href="https://leptos-auth-template-community.de/datenschutzerklaerung"/>
        <MetaLink rel="alternate" hreflang="en" href="https://leptos-auth-template-community.de/en/privacy"/>
        <h1>{t!(i18n, privacy.heading)}</h1>
    }
}
