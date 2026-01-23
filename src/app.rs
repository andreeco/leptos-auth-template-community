use crate::components::{footer::Footer, header::Header};
use crate::i18n::*; // from the include above
use crate::pages::{
    contact::Contact, home::Home, imprint::Imprint, login::LoginPage, not_found::NotFound,
    privacy::Privacy, protected::Protected,
};

use leptos::prelude::*;
use leptos_i18n_router::{i18n_path, I18nRoute};
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{components::*, StaticSegment};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    // let is_logged_in: Signal<Option<bool>> = Signal::derive(|| {
    //     Some(true)
    //     // Some(false)
    // });
    let is_logged_in = Resource::new(|| (), |_| crate::pages::protected::is_logged_in());

    view! {
             <I18nContextProvider>
            <Stylesheet id="leptos" href="/pkg/leptos-axum-login-try.css"/>
            <Title text="Willkommen / Welcome"/>
            <Header />
            <Router>
                <main>
                    <Routes fallback=|| view! { <NotFound/> }>
                        <I18nRoute<Locale, _, _> view=Outlet>
                            <Route path=StaticSegment("") view=Home/>
                            <Route
                                path=i18n_path!(Locale, |locale|
                                    td_string!(locale, contact_path))
                                view=Contact
                            />
                            <Route
                                path=i18n_path!(Locale, |locale|
                                    td_string!(locale, privacy_path))
                                view=Privacy
                            />
                            <Route
                                path=i18n_path!(Locale, |locale|
                                    td_string!(locale, imprint_path))
                                view=Imprint
                            />
    <Route path=StaticSegment("login") view=LoginPage/>
                        <ProtectedRoute
                                path=StaticSegment("protected")
                                view=Protected
                                condition=move || {
                                    match is_logged_in.get() {
                                        Some(Ok(value)) => Some(value),
                                        Some(Err(e)) => {
                                            leptos::logging::log!("is_logged_in resource error: {e}");
                                            Some(false)
                                        }
                                        None => None,
                                    }
                                }
                                redirect_path=move || "/login"
                                fallback=|| view! { <p>Checking login...</p> }
                            />
                        </I18nRoute<Locale, _, _>>
                    </Routes>
                </main>
            </Router>
            <Footer />
            </I18nContextProvider>
        }
}

#[server(LogoutUser)]
pub async fn logout_user() -> Result<(), ServerFnError> {
    use crate::auth::AuthSession;
    use axum::Extension;

    let res = leptos_axum::ResponseOptions::default();
    let Extension(mut auth): Extension<AuthSession> = leptos_axum::extract().await?;

    leptos_axum::redirect("/");

    match auth.logout().await {
        Ok(_) => Ok(()),
        Err(e) => {
            res.set_status(http::StatusCode::INTERNAL_SERVER_ERROR);
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}
