use crate::i18n::*;
use leptos::prelude::*;

#[component]
pub fn ContactButton() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <a href="mailto:kontakt@leptos-axum-login-try.de">
            <button>
                {t!(i18n, contact.button_label)}
            </button>
        </a>
    }
}
