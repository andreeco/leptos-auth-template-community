use crate::i18n::*;
use leptos::prelude::*;
use leptos_meta::{Link as MetaLink, Title};

#[component]
pub fn Imprint() -> impl IntoView {
    let i18n = use_i18n();
    let imprint_title = move || td_string!(i18n.get_locale(), imprint.title).to_string();

    view! {
    <Title text=imprint_title />
    <MetaLink rel="alternate" hreflang="de" href="https://leptos-auth-template-community.de/impressum"/>
    <MetaLink rel="alternate" hreflang="en" href="https://leptos-auth-template-community.de/en/imprint"/>
    <h1>{t!(i18n, imprint.heading)}</h1>
    }
}
