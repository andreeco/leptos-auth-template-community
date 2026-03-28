use crate::i18n::*;
use crate::i18n_utils::{AlternateLinks, AlternateRoute};
use leptos::prelude::*;
use leptos_meta::Title;

#[component]
pub fn Privacy() -> impl IntoView {
    let i18n = use_i18n();
    let locale_now = Signal::derive(move || i18n.get_locale());
    let privacy_title = move || td_string!(locale_now.get(), privacy.title).to_string();

    view! {
        <Title text=privacy_title />
        <AlternateLinks route=AlternateRoute::Privacy />
        <h1>{t!(i18n, privacy.heading)}</h1>
    }
}
