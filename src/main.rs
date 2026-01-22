#[cfg(feature = "ssr")]
mod auth;

#[cfg(feature = "ssr")]
use auth::{AuthSession, Backend, Credentials};

#[cfg(feature = "ssr")]
mod state;

#[cfg(feature = "ssr")]
use axum::{
    extract::Form,
    response::{Html, IntoResponse, Redirect},
    routing::get,
    Router,
};

#[cfg(feature = "ssr")]
use axum_login::AuthManagerLayerBuilder;

#[cfg(feature = "ssr")]
use tower_sessions::{MemoryStore, SessionManagerLayer};

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use crate::state::AppState;
    use leptos_axum_login_try::app::*;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};

    // Leptos SSR boilerplate
    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    let routes = generate_route_list(App);

    let app_state = AppState {
        leptos_options,
        routes: routes.clone(),
    };

    let backend = Backend::new();
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);
    let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

    let app = Router::new()
        .route("/login", get(login_page).post(do_login))
        .route("/logout", get(logout))
        .route("/protected", get(protected))
        .leptos_routes_with_context(
            &app_state,
            routes,
            {
                let state = app_state.clone();
                move || {
                    leptos::prelude::provide_context(state.clone());
                }
            },
            {
                let opts = app_state.leptos_options.clone();
                move || shell(opts.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler::<AppState, _>(shell))
        .layer(auth_layer)
        .with_state(app_state);

    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {}

#[cfg(feature = "ssr")]
#[axum::debug_handler]
async fn do_login(mut auth: AuthSession, Form(creds): Form<Credentials>) -> impl IntoResponse {
    if let Ok(Some(user)) = auth.authenticate(creds).await {
        if auth.login(&user).await.is_ok() {
            return Redirect::to("/protected").into_response();
        }
    }
    Redirect::to("/login").into_response()
}

#[cfg(feature = "ssr")]
async fn logout(mut auth: AuthSession) -> impl IntoResponse {
    let _ = auth.logout().await;
    Redirect::to("/login")
}

#[cfg(feature = "ssr")]
async fn protected(auth: AuthSession) -> impl IntoResponse {
    // if let Some(user) = auth.user().await { // does not work
    if let Some(user) = auth.user.clone() {
        Html(format!(
            "Hi {}, you are logged in! \
             <form action=\"/logout\"><button>Logout</button></form>",
            user.username
        ))
        .into_response()
    } else {
        Redirect::to("/login").into_response()
    }
}

#[cfg(feature = "ssr")]
async fn login_page() -> Html<&'static str> {
    Html(
        r#"
    <form method="POST" action="/login">
      <input name="username" placeholder="username" value="user">
      <input name="password" type="password" placeholder="password" value="password">
      <button>Log In</button>
    </form>
    "#,
    )
}

// This is the original code
/*
 * #[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
   leptos_axum_login_try::app::*;
    use axum::Router;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    let app = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}

*/
