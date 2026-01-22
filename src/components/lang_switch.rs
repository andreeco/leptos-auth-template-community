use crate::i18n::*;
use leptos::prelude::*;

#[component]
pub fn LangSwitch() -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <button on:click=move |_| {
            let new_locale = match i18n.get_locale() {
                Locale::en => Locale::de,
                Locale::de => Locale::en,
            };

            i18n.set_locale(new_locale);
        }>
            {move || match i18n.get_locale() {
                Locale::en => "Deutsch",    // Will say "Deutsch" when EN is active
                Locale::de => "English",    // Will say "English" when DE is active
            }}
        </button>
    }
}
