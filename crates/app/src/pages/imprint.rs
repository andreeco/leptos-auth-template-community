use crate::i18n::*;
use crate::i18n_utils::{AlternateLinks, AlternateRoute};
use leptos::prelude::*;
use leptos_meta::Title;

#[component]
pub fn Imprint() -> impl IntoView {
    let i18n = use_i18n();
    let imprint_title = move || td_string!(i18n.get_locale(), imprint.title).to_string();

    view! {
        <Title text=imprint_title />
        <AlternateLinks route=AlternateRoute::Imprint />
        <h1>{t!(i18n, imprint.heading)}</h1>
    }
}
