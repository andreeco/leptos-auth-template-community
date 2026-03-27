use crate::i18n::*;
use leptos::prelude::*;

#[component]
pub fn LangSwitch() -> impl IntoView {
    let i18n = use_i18n();

    let current = Signal::derive(move || i18n.get_locale());

    let on_change = move |ev: leptos::ev::Event| {
        let next = match event_target_value(&ev).as_str() {
            "de" => Locale::de,
            "en" => Locale::en,
            _ => Locale::en,
        };

        i18n.set_locale(next);
    };

    view! {
        <select
            class="lang-switch"
            aria-label="Language"
            prop:value=move || match current.get() {
                Locale::de => "de",
                Locale::en => "en",
            }
            on:change=on_change
        >
            <option value="de">"DE"</option>
            <option value="en">"EN"</option>
        </select>
    }
}
