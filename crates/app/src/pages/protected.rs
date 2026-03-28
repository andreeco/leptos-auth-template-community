use crate::contexts::AuthState;
use crate::i18n::*;
use leptos::prelude::*;
use leptos::task::spawn_local;

#[server(prefix = "/api/secure")]
pub async fn add_two(a: i32, b: i32) -> Result<i32, ServerFnError> {
    use crate::features::auth::AuthSession;
    use axum::Extension;

    let Extension(auth): Extension<AuthSession> = leptos_axum::extract().await?;

    if auth.user.is_none() {
        return Err(ServerFnError::ServerError("err_not_authenticated".into()));
    }

    Ok(a + b)
}

#[component]
pub fn Protected() -> impl IntoView {
    let i18n = use_i18n();
    let locale_now = Signal::derive(move || i18n.get_locale());
    let auth = expect_context::<AuthState>();
    let sum = RwSignal::new(None);
    let error = RwSignal::new(None);

    let compute = move |_| {
        let sum = sum.clone();
        let error = error.clone();
        spawn_local(async move {
            match add_two(21, 21).await {
                Ok(value) => {
                    sum.set(Some(value));
                    error.set(None);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    sum.set(None);
                }
            }
        });
    };

    view! {
        <h1>{t!(i18n, protected.title)}</h1>
        <p>
            {t!(i18n, protected.login_as)}
            {move || auth.username().unwrap_or_else(|| {
                td_string!(locale_now.get(), protected.not_authenticated).to_string()
            })}
        </p>
        <button on:click=compute>{t!(i18n, protected.button)}</button>
        <Show when=move || sum.get().is_some()>
            <p>
                {t!(i18n, protected.result)} {move || sum.get().unwrap().to_string()}
            </p>
        </Show>
        <Show when=move || error.get().is_some()>
            <p style="color:red">
                {move || {
                    let raw = error.get().unwrap_or_default();
                    match raw.as_str() {
                        "err_not_authenticated" => {
                            td_string!(locale_now.get(), protected.not_authenticated).to_string()
                        }
                        _ => raw,
                    }
                }}
            </p>
        </Show>
    }
}
