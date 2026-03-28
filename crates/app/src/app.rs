use crate::contexts::{AuthState, CsrfContext, Permission, UserSummary};
use crate::components::{footer::Footer, header::Header};
use crate::i18n::*;
use crate::i18n_utils::localized_path;
use crate::pages::{
    account::{AccountPage, AccountPasswordPage, AccountProfilePage, AccountWebauthnPage},
    admin::AdminPage, contact::Contact, home::Home, imprint::Imprint, login::LoginPage,
    logout::LogoutPage, not_found::NotFound, privacy::Privacy, protected::Protected,
};
use leptos::prelude::*;
use leptos_i18n_router::{i18n_path, I18nRoute};
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::*,
    lazy_route, Lazy, LazyRoute,
};

use std::collections::HashSet;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options islands=true/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
fn GuardCheckingLoginFallback() -> impl IntoView {
    let i18n = use_i18n();
    view! { <p>{t!(i18n, protected.guard_checking_login)}</p> }
}

#[component]
fn GuardCheckingAccountAccessFallback() -> impl IntoView {
    let i18n = use_i18n();
    view! { <p>{t!(i18n, protected.guard_checking_account_access)}</p> }
}

#[component]
fn GuardCheckingAdminAccessFallback() -> impl IntoView {
    let i18n = use_i18n();
    view! { <p>{t!(i18n, protected.guard_checking_admin_access)}</p> }
}

#[component]
fn AppRoutes(auth: AuthState) -> impl IntoView {
    let i18n = use_i18n();
    let login_redirect_path = Signal::derive(move || {
        localized_path(
            i18n.get_locale(),
            td_string!(i18n.get_locale(), routes.login_path),
        )
    });
    let account_password_redirect_path = Signal::derive(move || {
        localized_path(
            i18n.get_locale(),
            td_string!(i18n.get_locale(), routes.account_password_path),
        )
    });
    let protected_redirect_path = Signal::derive(move || {
        if auth.ready.get() && auth.logged_in() && auth.requires_password_reset() {
            account_password_redirect_path.get()
        } else {
            login_redirect_path.get()
        }
    });

    view! {
        <Routes fallback=|| view! { <NotFound/> }>
            <I18nRoute<Locale, _,_> view=|| view! {
                <crate::contexts::ApplySsrAuthSnapshot/>
                <crate::contexts::EnsureAuthSnapshot/>
                <crate::contexts::EnsureCsrfToken/>
                <Header />
                <main>
                    <Outlet />
                </main>
                <Footer />
            }>
                <Route path=i18n_path!(Locale, |locale| td_string!(locale, routes.home_path)) view=Home/>
                <Route path=i18n_path!(Locale, |locale| td_string!(locale, routes.contact_path)) view={Lazy::<ContactRoute>::new()}/>
                <Route path=i18n_path!(Locale, |locale| td_string!(locale, routes.privacy_path)) view={Lazy::<PrivacyRoute>::new()}/>
                <Route path=i18n_path!(Locale, |locale| td_string!(locale, routes.imprint_path)) view={Lazy::<ImprintRoute>::new()}/>
                <Route path=i18n_path!(Locale, |locale| td_string!(locale, routes.login_path)) view={Lazy::<LoginRoute>::new()}/>
                <Route path=i18n_path!(Locale, |locale| td_string!(locale, routes.logout_path)) view={Lazy::<LogoutRoute>::new()}/>
                <ProtectedRoute
                    path=i18n_path!(Locale, |locale| td_string!(locale, routes.protected_path))
                    view=Protected
                    condition=move || {
                        if !auth.ready.get() {
                            None
                        } else {
                            Some(auth.logged_in() && !auth.requires_password_reset())
                        }
                    }
                    redirect_path=move || protected_redirect_path.get()
                    fallback=|| view! { <GuardCheckingLoginFallback /> }
                />
                <ProtectedRoute
                    path=i18n_path!(Locale, |locale| td_string!(locale, routes.account_path))
                    view=AccountPage
                    condition=move || {
                        if !auth.ready.get() {
                            None
                        } else {
                            Some(auth.logged_in() && !auth.requires_password_reset())
                        }
                    }
                    redirect_path=move || protected_redirect_path.get()
                    fallback=|| view! { <GuardCheckingAccountAccessFallback /> }
                />
                <ProtectedRoute
                    path=i18n_path!(Locale, |locale| td_string!(locale, routes.account_profile_path))
                    view=AccountProfilePage
                    condition=move || {
                        if !auth.ready.get() {
                            None
                        } else {
                            Some(auth.logged_in() && !auth.requires_password_reset())
                        }
                    }
                    redirect_path=move || protected_redirect_path.get()
                    fallback=|| view! { <GuardCheckingAccountAccessFallback /> }
                />
                <ProtectedRoute
                    path=i18n_path!(Locale, |locale| td_string!(locale, routes.account_password_path))
                    view=AccountPasswordPage
                    condition=move || {
                        if !auth.ready.get() {
                            None
                        } else {
                            Some(auth.logged_in())
                        }
                    }
                    redirect_path=move || login_redirect_path.get()
                    fallback=|| view! { <GuardCheckingAccountAccessFallback /> }
                />
                <ProtectedRoute
                    path=i18n_path!(Locale, |locale| td_string!(locale, routes.account_webauthn_path))
                    view=AccountWebauthnPage
                    condition=move || {
                        if !auth.ready.get() {
                            None
                        } else {
                            Some(auth.logged_in() && !auth.requires_password_reset())
                        }
                    }
                    redirect_path=move || protected_redirect_path.get()
                    fallback=|| view! { <GuardCheckingAccountAccessFallback /> }
                />
                <ProtectedRoute
                    path=i18n_path!(Locale, |locale| td_string!(locale, routes.admin_path))
                    view=AdminPage
                    condition=move || {
                        if !auth.ready.get() {
                            None
                        } else {
                            Some(auth.is_admin() && !auth.requires_password_reset())
                        }
                    }
                    redirect_path=move || protected_redirect_path.get()
                    fallback=|| view! { <GuardCheckingAdminAccessFallback /> }
                />
            </I18nRoute<Locale, _,_>>
        </Routes>
    }
}

macro_rules! define_lazy_route {
    ($name:ident, $component:ident) => {
        struct $name;

        #[lazy_route]
        impl LazyRoute for $name {
            fn data() -> Self {
                Self
            }

            fn view(this: Self) -> AnyView {
                let _ = this;
                view! { <$component/> }.into_any()
            }
        }
    };
}

define_lazy_route!(ContactRoute, Contact);
define_lazy_route!(PrivacyRoute, Privacy);
define_lazy_route!(ImprintRoute, Imprint);
define_lazy_route!(LoginRoute, LoginPage);
define_lazy_route!(LogoutRoute, LogoutPage);

#[island]
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

    let auth = expect_context::<AuthState>();

    let csrf_sig = RwSignal::new(None::<String>);
    provide_context(CsrfContext(csrf_sig));

    let csrf_refresh = RwSignal::new(());
    provide_context(csrf_refresh);

    view! {
        <I18nContextProvider>
            <Stylesheet id="leptos" href="/pkg/leptos-auth-template-community.css"/>
            <Title text="Willkommen / Welcome"/>
            <Router>
                <AppRoutes auth=auth />
            </Router>
        </I18nContextProvider>
    }
}
