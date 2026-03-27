use crate::components::contact_button::ContactButton;
use crate::i18n::*;
use leptos::prelude::*;
use leptos_meta::{Link as MetaLink, Title};

#[component]
pub fn Contact() -> impl IntoView {
    let i18n = use_i18n();
    let contact_title = td_string!(i18n.get_locale_untracked(), contact.title).to_string();

    view! {
        <Title text={contact_title} />
        <MetaLink rel="alternate" hreflang="de" href="https://leptos-auth-template-community.de/kontakt"/>
        <MetaLink rel="alternate" hreflang="en" href="https://leptos-auth-template-community.de/en/contact"/>
        <h1>{t!(i18n, contact.title)}</h1>
        <p>
            {t!(i18n, contact.email)}
            <a href="mailto:kontakt@leptos-auth-template-community.de">kontakt@leptos-auth-template-community.de</a>
        </p>
        <ContactButton />
    }
}
