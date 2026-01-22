use crate::i18n::*;
use leptos::prelude::*;
use leptos_meta::{Link as MetaLink, Title};

#[component]
pub fn Imprint() -> impl IntoView {
    let i18n = use_i18n();
    view! {
    <Title text={t_string!(i18n, imprint.title)} />
    <MetaLink rel="alternate" hreflang="de" href="https://leptos-axum-login-try.de/impressum"/>
    <MetaLink rel="alternate" hreflang="en" href="https://leptos-axum-login-try.de/en/imprint"/>
    <h1>{t!(i18n, imprint.heading)}</h1>
    }
}
