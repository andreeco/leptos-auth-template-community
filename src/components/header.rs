use crate::auth_state::AuthState;
use crate::components::lang_switch::LangSwitch;
use crate::i18n::*;
use leptos::prelude::*;

#[component]
pub fn Header() -> impl IntoView {
    let i18n = use_i18n();
    let auth = expect_context::<AuthState>();
    view! {
        <header>
            <nav>
                <a href="/">{t!(i18n, header.home)}</a> |
                <a href="/kontakt">{t!(i18n, header.contact)}</a> |

                <Show when=move || auth.ready.get() fallback=|| ()>
                    <Show when=move || !auth.logged_in() fallback=|| ()>
                        <a href="/login">{t!(i18n, header.login)}</a> |
                    </Show>
                    <Show when=move || auth.logged_in() fallback=|| ()>
                        <a href="/protected">{t!(i18n, header.protected)}</a> |
                        <Show when=move || auth.is_admin() fallback=|| ()>
                            <a href="/admin">{t!(i18n, header.admin)}</a> |
                        </Show>
                        <a href="/logout">{t!(i18n, header.logout)}</a> |
                    </Show>
                </Show>

                <LangSwitch />
            </nav>
            <hr/>
        </header>
    }
}
