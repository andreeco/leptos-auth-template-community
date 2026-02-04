use crate::i18n::*;
use leptos::prelude::*;
use leptos_meta::{Link as MetaLink, Title};

#[component]
pub fn Home() -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <Title text={t_string!(i18n, home.title)} />
        <MetaLink rel="alternate" hreflang="de" href="https://leptos-axum-login-try.de/"/>
        <MetaLink rel="alternate" hreflang="en" href="https://leptos-axum-login-try.de/en"/>

        <h1>{t!(i18n, home.heading)}</h1>
        <p>{t!(i18n, home.intro)}</p>

        <h2>{t!(i18n, home.what_title)}</h2>
        <ul>
            <li>{t!(i18n, home.what_1)}</li>
            <li>{t!(i18n, home.what_2)}</li>
            <li>{t!(i18n, home.what_3)}</li>
            <li>{t!(i18n, home.what_4)}</li>
            <li>{t!(i18n, home.what_5)}</li>
        </ul>

        <h2>{t!(i18n, home.how_title)}</h2>
        <ol>
            <li>{t!(i18n, home.how_1)}</li>
            <li>{t!(i18n, home.how_2)}</li>
            <li>{t!(i18n, home.how_3)}</li>
        </ol>

        <h2>{t!(i18n, home.rules_title)}</h2>
        <ul>
            <li>{t!(i18n, home.rules_1)}</li>
            <li>{t!(i18n, home.rules_2)}</li>
            <li>{t!(i18n, home.rules_3)}</li>
            <li>{t!(i18n, home.rules_4)}</li>
        </ul>

        <h2>{t!(i18n, home.paths_title)}</h2>
        <ul>
            <li>{t!(i18n, home.paths_1)}</li>
            <li>{t!(i18n, home.paths_2)}</li>
            <li>{t!(i18n, home.paths_3)}</li>
        </ul>

        <h2>{t!(i18n, home.demo_warning_title)}</h2>
        <p style="color: orange;">
            <strong>{t!(i18n, home.demo_warning)}</strong>
        </p>
    }
}
