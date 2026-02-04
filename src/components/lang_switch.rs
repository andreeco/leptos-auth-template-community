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
                Locale::en => t!(i18n, header.german_label)(),
                Locale::de => t!(i18n, header.english_label)(),
            }}
        </button>
    }
}
