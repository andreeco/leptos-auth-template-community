use crate::components::contact_button::ContactButton;
use crate::i18n::*;
use leptos::prelude::*;
use leptos_meta::{Link as MetaLink, Title};

#[component]
pub fn Contact() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <Title text={t_string!(i18n, contact.title)} />
        <MetaLink rel="alternate" hreflang="de" href="https://leptos-axum-login-try.de/kontakt"/>
        <MetaLink rel="alternate" hreflang="en" href="https://leptos-axum-login-try.de/en/contact_me"/>
        <h1>{t!(i18n, contact.title)}</h1>
        <p>
            {t!(i18n, contact.email)}
            <a href="mailto:kontakt@leptos-axum-login-try.de">kontakt@leptos-axum-login-try.de</a>
        </p>
        <ContactButton />
    }
}

// This is expected from leptos warning but is not needed!!
// #[component]
// pub fn Contact() -> impl IntoView {
//     let i18n = use_i18n();
//     view! {
//         <Title text={move || t_string!(i18n, contact.title)} />
//         <MetaLink rel="alternate" hreflang="de" href="https://leptos-axum-login-try.de/kontakt"/>
//         <MetaLink rel="alternate" hreflang="en" href="https://leptos-axum-login-try.de/en/contact_me"/>
//         <h1>{move || t!(i18n, contact.title)}</h1>
//         <p>
//             {move || t!(i18n, contact.email)}
//             <a href="mailto:kontakt@leptos-axum-login-try.de">kontakt@leptos-axum-login-try.de</a>
//         </p>
//         <a href="/en/contact_me">{move || t!(i18n, contact.english_version)}</a>
//     }
// }
