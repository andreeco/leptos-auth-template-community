#[cfg(feature = "ssr")]
mod auth;

#[cfg(feature = "ssr")]
use auth::Backend;

#[cfg(feature = "ssr")]
mod state;

#[cfg(feature = "ssr")]
use axum::Router;

#[cfg(feature = "ssr")]
use axum_login::AuthManagerLayerBuilder;

#[cfg(feature = "ssr")]
use tower_sessions::{MemoryStore, SessionManagerLayer};

use axum::extract::Extension;

pub async fn debug_extensions(maybe_auth: Option<Extension<crate::auth::AuthSession>>) -> String {
    if let Some(auth) = maybe_auth {
        format!("AuthSession IS present! User: {:?}", auth.user)
    } else {
        "AuthSession not present!".to_string()
    }
}

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use crate::state::AppState;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use leptos_axum_login_try::app::*; // App, shell

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let app_state = AppState {
        leptos_options,
        routes: routes.clone(),
    };

    // --- axum-login setup ---
    let backend = Backend::new();
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);
    let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

    use tower_cookies::CookieManagerLayer;

    let app = Router::new()
        .route(
            "/api/debug-extensions",
            axum::routing::get(debug_extensions),
        )
        .leptos_routes_with_context(
            &app_state,
            routes,
            {
                let state = app_state.clone();
                move || {
                    // provide shared state (DB, config, etc) to leptos
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
        .layer(CookieManagerLayer::new())
        .with_state(app_state);

    leptos::logging::log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {}

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
