use crate::i18n::*;
use crate::i18n_utils::{locale_code, locale_from_code, locale_label, supported_locales};
use leptos::prelude::*;

#[component]
pub fn LangSwitch() -> impl IntoView {
    let i18n = use_i18n();

    let current = Signal::derive(move || i18n.get_locale());

    let on_change = move |ev: leptos::ev::Event| {
        let raw = event_target_value(&ev);
        let next = locale_from_code(&raw).unwrap_or_else(|| current.get_untracked());
        i18n.set_locale(next);
    };

    view! {
        <select
            class="lang-switch"
            aria-label="Language"
            prop:value=move || locale_code(current.get())
            on:change=on_change
        >
            {supported_locales()
                .iter()
                .copied()
                .map(|locale| {
                    view! { <option value=locale_code(locale)>{locale_label(locale)}</option> }
                })
                .collect_view()}
        </select>
    }
}
