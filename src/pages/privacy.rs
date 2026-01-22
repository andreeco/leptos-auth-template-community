use crate::i18n::*;
use leptos::prelude::*;
use leptos_meta::{Link as MetaLink, Title};

#[component]
pub fn Privacy() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <Title text={t_string!(i18n, privacy.title)} />
        <MetaLink rel="alternate" hreflang="de" href="https://leptos-axum-login-try.de/datenschutzerklaerung"/>
        <MetaLink rel="alternate" hreflang="en" href="https://leptos-axum-login-try.de/en/privacy"/>
        <h1>{t!(i18n, privacy.heading)}</h1>
    }
}
