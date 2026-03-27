use crate::i18n::*;
use leptos::prelude::*;
use leptos_meta::Title;

#[component]
pub fn Home() -> impl IntoView {
    let i18n = use_i18n();
    let home_title = td_string!(i18n.get_locale_untracked(), home.title).to_string();

    view! {
        <Title text={home_title} />
        <section>
            <h1>{t!(i18n, home.heading)}</h1>
        </section>
    }
}
