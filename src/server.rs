use axum::{
    extract::{Request, State},
    response::IntoResponse,
    routing::get,
    Extension, Router,
};
use axum_login::{login_required, AuthManagerLayerBuilder};
use http::header::{HeaderName, HeaderValue};
use leptos::prelude::{provide_context, LeptosOptions};
use leptos_axum::{handle_server_fns_with_context, LeptosRoutes};
use leptos_axum_login_try::{
    auth::{AuthSession, Backend},
    csrf::{csrf_for_ssr, provide_csrf_context, CsrfToken},
    state::AppState,
};
use std::{net::SocketAddr, sync::Arc};
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
};
use tower_http::{
    limit::RequestBodyLimitLayer, set_header::SetResponseHeaderLayer, trace::TraceLayer,
};
use tower_sessions::{cookie::SameSite, MemoryStore, Session, SessionManagerLayer};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AppEnv {
    Dev,
    Test,
    Prod,
}

fn app_env() -> AppEnv {
    match std::env::var("LEPTOS_ENV").as_deref() {
        Ok("prod") | Ok("production") => AppEnv::Prod,
        Ok("test") => AppEnv::Test,
        _ => AppEnv::Dev,
    }
}

fn secure_cookies(env: AppEnv) -> bool {
    env == AppEnv::Prod
}

fn rate_limits(env: AppEnv) -> (u64, u32) {
    match env {
        AppEnv::Prod => (5u64, 10u32),
        _ => (50u64, 100u32),
    }
}

fn csp_value(env: AppEnv) -> &'static str {
    match env {
        AppEnv::Prod => {
            "default-src 'self'; \
             base-uri 'self'; \
             object-src 'none'; \
             frame-ancestors 'none'; \
             img-src 'self' data:; \
             style-src 'self' 'unsafe-inline'; \
             script-src 'self'; \
             connect-src 'self';"
        }
        _ => {
            "default-src 'self' 'unsafe-inline' 'unsafe-eval' data: blob:; \
             img-src 'self' data: blob:; \
             style-src 'self' 'unsafe-inline'; \
             script-src 'self' 'unsafe-inline' 'unsafe-eval'; \
             connect-src 'self' ws: wss:;"
        }
    }
}

async fn server_fn_handler(
    State(app_state): State<AppState>,
    Extension(auth_session): Extension<AuthSession>,
    request: Request<axum::body::Body>,
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

async fn leptos_routes_handler(
    Extension(auth_session): Extension<AuthSession>,
    Extension(session): Extension<Session>,
    State(app_state): State<AppState>,
    axum::extract::State(leptos_options): axum::extract::State<LeptosOptions>,
    request: Request<axum::body::Body>,
) -> axum::response::Response {
    let shell_options = leptos_options.clone();

    let csrf: CsrfToken = csrf_for_ssr(&session).await.unwrap_or(CsrfToken {
        token: String::new(),
    });

    leptos_axum::render_app_async_with_context(
        move || {
            provide_context(leptos_options.clone());
            provide_context(app_state.clone());
            provide_context(auth_session.clone());
            provide_csrf_context(&csrf);
        },
        move || leptos_axum_login_try::app::shell(shell_options.clone()),
    )(request)
    .await
    .into_response()
}

pub async fn run() {
    let env = app_env();

    match env {
        AppEnv::Prod => println!("Production: Secure cookies enabled."),
        AppEnv::Test => println!("Test mode: Secure cookies disabled."),
        AppEnv::Dev => eprintln!("Development: Secure cookies disabled! NOT SAFE FOR PRODUCTION."),
    }

    // --- Leptos config ---
    use leptos_axum::generate_route_list;
    use leptos_axum_login_try::app::{shell, App};

    let conf = leptos::config::get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;

    let routes = generate_route_list(App);
    let app_state = AppState {
        leptos_options: leptos_options.clone(),
        routes: routes.clone(),
    };

    // --- Auth/session ---
    let backend = Backend::new();

    let session_layer = SessionManagerLayer::new(MemoryStore::default())
        .with_secure(secure_cookies(env))
        .with_http_only(true)
        .with_same_site(SameSite::Lax);

    let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

    // --- Middleware layers ---
    let trace_layer = TraceLayer::new_for_http();
    let body_limit_layer = RequestBodyLimitLayer::new(1024 * 1024);

    let (per_second, burst_size) = rate_limits(env);
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .key_extractor(SmartIpKeyExtractor)
            .per_second(per_second)
            .burst_size(burst_size)
            .finish()
            .unwrap(),
    );
    let rate_limit_layer = GovernorLayer::new(governor_conf);

    let nosniff = SetResponseHeaderLayer::overriding(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    let referrer_policy = SetResponseHeaderLayer::overriding(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    let permissions_policy = SetResponseHeaderLayer::overriding(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
    );
    let csp = SetResponseHeaderLayer::overriding(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_str(csp_value(env)).unwrap(),
    );

    // --- Routes ---
    let public_api = Router::new().route(
        "/api/{*fn_name}",
        get(server_fn_handler).post(server_fn_handler),
    );

    let app = Router::new()
        .merge(public_api)
        .route(
            "/api/secure/{*fn_name}",
            get(server_fn_handler)
                .post(server_fn_handler)
                .route_layer(login_required!(Backend)),
        )
        .leptos_routes_with_handler(routes, get(leptos_routes_handler))
        .fallback(leptos_axum::file_and_error_handler::<AppState, _>(shell))
        .layer(rate_limit_layer)
        .layer(body_limit_layer)
        .layer(nosniff)
        .layer(referrer_policy)
        .layer(permissions_policy)
        .layer(csp)
        .layer(trace_layer)
        .layer(auth_layer)
        .with_state(app_state);

    // --- Serve ---
    println!("listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    // Needed for tower_governor (peer-ip fallback) and generally useful:
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
