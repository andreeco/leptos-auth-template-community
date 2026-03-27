use crate::i18n::*;
use leptos::prelude::*;

#[component]
pub fn NotFound() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <h1>{t!(i18n, not_found.heading)}</h1>
    }
}
