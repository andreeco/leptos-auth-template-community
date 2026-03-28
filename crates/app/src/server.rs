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
use leptos_auth_template_community::{
    contexts::{csrf_for_ssr, provide_csrf_context, AuthSnapshot, CsrfToken, UserSummary},
    features::auth::{AuthSession, Backend},
    state::AppState,
};
use sea_orm::Database;
use std::{net::SocketAddr, sync::Arc};
use url::Url;
use webauthn_rs::WebauthnBuilder;
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
             script-src 'self' 'unsafe-inline'; \
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

fn hsts_value(env: AppEnv) -> &'static str {
    match env {
        AppEnv::Prod => "max-age=31536000; includeSubDomains",
        _ => "max-age=0",
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

fn make_auth_snapshot(auth_session: &AuthSession) -> AuthSnapshot {
    let user = auth_session.user.clone().map(|u| UserSummary {
        id: u.id,
        username: u.username,
        roles: u.roles,
        password_reset_required: u.password_reset_required,
    });

    let permissions = auth_session
        .user
        .clone()
        .map(|u| u.permissions)
        .unwrap_or_default();

    AuthSnapshot { user, permissions }
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
    let auth_snapshot = make_auth_snapshot(&auth_session);

    leptos_axum::render_app_async_with_context(
        move || {
            provide_context(leptos_options.clone());
            provide_context(app_state.clone());
            provide_context(auth_session.clone());
            provide_context(auth_snapshot.clone());
            provide_csrf_context(&csrf);
        },
        move || leptos_auth_template_community::app::shell(shell_options.clone()),
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
    use leptos_auth_template_community::app::{shell, App};

    let conf = leptos::config::get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;

    let routes = generate_route_list(App);

    // --- DB ---
    let db_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://app.sqlite?mode=rwc".into());
    let db = Database::connect(&db_url).await.expect("DB connect failed");

    // --- WebAuthn (build once) ---
    let rp_origin =
        std::env::var("WEBAUTHN_RP_ORIGIN").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let rp_id = std::env::var("WEBAUTHN_RP_ID").unwrap_or_else(|_| "localhost".to_string());
    let rp_name = std::env::var("WEBAUTHN_RP_NAME").unwrap_or_else(|_| "example-app".to_string());

    let rp_origin = Url::parse(&rp_origin).expect("Invalid WEBAUTHN_RP_ORIGIN");
    let webauthn = WebauthnBuilder::new(&rp_id, &rp_origin)
        .expect("Invalid WebAuthn RP config")
        .rp_name(&rp_name)
        .build()
        .expect("Failed to build WebAuthn");

    let app_state = AppState {
        leptos_options: leptos_options.clone(),
        routes: routes.clone(),
        db,
        webauthn,
    };

    // --- Auth/session ---
    let backend = Backend::new(app_state.db.clone());

    let session_layer = SessionManagerLayer::new(MemoryStore::default())
        .with_secure(secure_cookies(env))
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_expiry(tower_sessions::Expiry::OnInactivity(
            tower_sessions::cookie::time::Duration::hours(8),
        ))
        .with_always_save(true);

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
    let hsts = SetResponseHeaderLayer::overriding(
        HeaderName::from_static("strict-transport-security"),
        HeaderValue::from_static(hsts_value(env)),
    );

    // --- Routes ---
    let api = Router::new()
        .route(
            "/api/{*fn_name}",
            get(server_fn_handler).post(server_fn_handler),
        )
        .route(
            "/api/secure/{*fn_name}",
            get(server_fn_handler)
                .post(server_fn_handler)
                .route_layer(login_required!(Backend)),
        )
        .layer(rate_limit_layer);

    let app = Router::new()
        .merge(api)
        .leptos_routes_with_handler(routes, get(leptos_routes_handler))
        .fallback(leptos_axum::file_and_error_handler::<AppState, _>(shell))

        .layer(body_limit_layer)
        .layer(nosniff)
        .layer(referrer_policy)
        .layer(permissions_policy)
        .layer(csp)
        .layer(hsts)
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
