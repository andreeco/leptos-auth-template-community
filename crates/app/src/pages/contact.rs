use crate::components::contact_button::ContactButton;
use crate::i18n::*;
use crate::i18n_utils::{AlternateLinks, AlternateRoute};
use leptos::prelude::*;
use leptos_meta::Title;

#[component]
pub fn Contact() -> impl IntoView {
    let i18n = use_i18n();
    let locale_now = Signal::derive(move || i18n.get_locale());
    let contact_title = move || td_string!(locale_now.get(), contact.title).to_string();

    view! {
        <Title text=contact_title />
        <AlternateLinks route=AlternateRoute::Contact />
        <h1>{t!(i18n, contact.title)}</h1>
        <p>
            {t!(i18n, contact.email)}
            <a href="mailto:kontakt@leptos-auth-template-community.de">kontakt@leptos-auth-template-community.de</a>
        </p>
        <ContactButton />
    }
}
