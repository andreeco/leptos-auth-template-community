use crate::auth_state::{auth_snapshot, AuthSnapshot, AuthState, Permission, UserSummary};
use crate::components::{footer::Footer, header::Header};
use crate::csrf::CsrfContext;
use crate::i18n::*;
use crate::pages::{
    admin::AdminPage, contact::Contact, home::Home, imprint::Imprint, login::LoginPage,
    logout::LogoutPage, not_found::NotFound, privacy::Privacy, protected::Protected,
};
use leptos::prelude::*;
use leptos_i18n_router::{i18n_path, I18nRoute};
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::*;
use std::collections::HashSet;

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

    let (ready, set_ready) = signal(false);
    let (user, set_user) = signal::<Option<UserSummary>>(None);
    let (permissions, set_permissions) = signal::<HashSet<Permission>>(HashSet::new());

    provide_context(AuthState {
        ready,
        set_ready,
        user,
        set_user,
        permissions,
        set_permissions,
    });

    let init_auth = Resource::new(|| (), |_| auth_snapshot());

    Effect::new(move |_| {
        if let Some(Ok(AuthSnapshot { user, permissions })) = init_auth.get() {
            set_user.set(user);
            set_permissions.set(permissions);
            set_ready.set(true);
        }
    });

    let auth = expect_context::<AuthState>();

    let csrf_sig = RwSignal::new(None::<String>);
    provide_context(CsrfContext(csrf_sig));

    let csrf_refresh = RwSignal::new(());
    provide_context(csrf_refresh);

    let csrf_res = LocalResource::new(move || {
        csrf_refresh.get();
        crate::csrf::get_csrf_token()
    });
    Effect::new(move |_| {
        if let Some(Ok(tok)) = csrf_res.get() {
            csrf_sig.set(Some(tok.token));
        }
    });

    view! {
        <I18nContextProvider>
            <Stylesheet id="leptos" href="/pkg/leptos-axum-login-try.css"/>
            <Title text="Willkommen / Welcome"/>
            <Header />
            <Router>
                <main>
                    <Routes fallback=|| view! { <NotFound/> }>
                        <I18nRoute<Locale, _,_> view=Outlet>
                            <Route path=i18n_path!(Locale, |locale| td_string!(locale, routes.home_path)) view=Home/>
                            <Route path=i18n_path!(Locale, |locale| td_string!(locale, routes.contact_path)) view=Contact/>
                            <Route path=i18n_path!(Locale, |locale| td_string!(locale, routes.privacy_path)) view=Privacy/>
                            <Route path=i18n_path!(Locale, |locale| td_string!(locale, routes.imprint_path)) view=Imprint/>
                            <Route path=i18n_path!(Locale, |locale| td_string!(locale, routes.login_path)) view=LoginPage/>
                            <Route path=i18n_path!(Locale, |locale| td_string!(locale, routes.logout_path)) view=LogoutPage/>
                            <ProtectedRoute
                                path=i18n_path!(Locale, |locale| td_string!(locale, routes.protected_path))
                                view=Protected
                                condition=move || {
                                    if !auth.ready.get() {
                                        None
                                    } else {
                                        Some(auth.logged_in())
                                    }
                                }
                                redirect_path=move || i18n_path!(Locale, |locale| td_string!(locale, routes.login_path))
                                fallback=|| view! { <p>Checking login...</p> }
                            />
                            <ProtectedRoute
                                path=i18n_path!(Locale, |locale| td_string!(locale, routes.admin_path))
                                view=AdminPage
                                condition=move || {
                                    if !auth.ready.get() {
                                        None
                                    } else {
                                        Some(auth.is_admin())
                                    }
                                }
                                redirect_path=move || i18n_path!(Locale, |locale| td_string!(locale, routes.login_path))
                                fallback=|| view! { <p>Checking admin access...</p> }
                            />
                        </I18nRoute<Locale, _,_>>
                    </Routes>
                </main>
            </Router>
            <Footer />
        </I18nContextProvider>
    }
}
