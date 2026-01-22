use crate::components::contact_button::ContactButton;
use crate::i18n::*;
use leptos::prelude::*;
use leptos_meta::{Link as MetaLink, Title};

#[component]
pub fn Home() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <Title text={t_string!(i18n, home.title)} />
        <MetaLink rel="alternate" hreflang="de" href="https://leptos-axum-login-try.de/"/>
        <MetaLink rel="alternate" hreflang="en" href="https://leptos-axum-login-try.de/en"/>
        <h1>{t!(i18n, home.heading)}</h1>

        <ContactButton />
    }
}
