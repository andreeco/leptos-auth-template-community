#[cfg(feature = "ssr")]
use axum::Router;

#[cfg(feature = "ssr")]
use axum_login::AuthManagerLayerBuilder;

#[cfg(feature = "ssr")]
use tower_sessions::{MemoryStore, SessionManagerLayer};

use axum::extract::Extension;

#[cfg(feature = "ssr")]
use leptos_axum_login_try::{auth::{AuthSession, Backend}, state::AppState};
#[cfg(feature = "ssr")]
use axum::{
    extract::{Request, State},
    response::IntoResponse,
    routing::get,
};
#[cfg(feature = "ssr")]
use leptos::prelude::provide_context;
#[cfg(feature = "ssr")]
use leptos_axum::handle_server_fns_with_context;

#[cfg(feature = "ssr")]
use axum::body::Body as AxumBody;
use leptos::prelude::LeptosOptions;

#[cfg(feature = "ssr")]
async fn server_fn_handler(
    State(app_state): State<AppState>,
    auth_session: AuthSession,
    request: Request<AxumBody>,
) -> impl IntoResponse {
    handle_server_fns_with_context(
        move || {
            provide_context(app_state.clone());
            provide_context(auth_session.clone());
        },
        request,
    )
    .await
}

#[cfg(feature = "ssr")]
pub async fn leptos_routes_handler(
    auth_session: AuthSession,
    State(app_state): State<AppState>,
    axum::extract::State(option): axum::extract::State<LeptosOptions>,
    request: Request<AxumBody>,
) -> axum::response::Response {
    let leptos_options = option.clone();
    let handler = leptos_axum::render_app_async_with_context(
        move || {
            provide_context(option.clone());
            provide_context(app_state.clone());
            provide_context(auth_session.clone());
            //provide_context(app_state.pool.clone());
        },
        move || leptos_axum_login_try::app::shell(leptos_options.clone()),
    );

    handler(request).await.into_response()
}

pub async fn debug_extensions(maybe_auth: Option<Extension<leptos_axum_login_try::auth::AuthSession>>) -> String {
    if let Some(auth) = maybe_auth {
        format!("AuthSession IS present! User: {:?}", auth.user)
    } else {
        "AuthSession not present!".to_string()
    }
}

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
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

    // dont think you need this
    //use tower_cookies::CookieManagerLayer;

    let app = Router::new()
        .route(
            "/api/{*fn_name}",
            get(server_fn_handler).post(server_fn_handler),
        )
        .leptos_routes_with_handler(routes, get(leptos_routes_handler))
        .fallback(leptos_axum::file_and_error_handler::<AppState, _>(shell))
        .layer(auth_layer)
        //.layer(CookieManagerLayer::new())
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
