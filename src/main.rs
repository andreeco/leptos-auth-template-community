#[cfg(feature = "ssr")]
mod server;

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    server::run().await;
}

#[cfg(not(feature = "ssr"))]
pub fn main() {}

// Authorization footgun: /api/secure/* is login-only; admin must be enforced per fn or via admin route group.

// ### Two ways to enforce auth for Leptos `#[server]` functions (what you did vs. `MiddlewareSet`)

// #### 1) What you did (Axum router-level middleware; recommended with Axum)
// **Idea:** Treat server functions as ordinary HTTP endpoints and protect whole URL groups in Axum.

// - You mounted server fns at:
//   - public: `/api/{*fn_name}`
//   - authenticated: `/api/secure/{*fn_name}`
// - You applied Axum middleware to the secure group:
//   - `login_required!(Backend)` (from `axum-login`)
// - You made “secure” server functions use:
//   - `#[server(prefix="/api/secure")]`

// **Result:**
// - Any server fn under `/api/secure/*` requires a valid session.
// - Role/permission checks are still done inside server fn bodies (e.g. `admin_add_two` checks `Role::Admin`).

// **Pros:**
// - Central, obvious, idiomatic Axum.
// - Works with any extractor/session/auth crate.
// - Easy to reason about and test.

// ---

// #### 2) How it would look with `server_fn::MiddlewareSet` (per-function middleware)
// **Idea:** Each server function can provide its own middleware stack, independent of how the Axum router groups routes.

// Conceptually you’d do something like:

// - Define a layer that checks authentication/authorization.
// - Attach it to a specific server fn via its generated `ServerFn` type’s middleware hook.

// Pseudo-example (illustrative, not copy/paste-ready because the exact trait hooks depend on server_fn version/integration):

// ```rust
// use leptos::prelude::*;
// use leptos::server_fn::MiddlewareSet;
// use std::sync::Arc;
// use tower_layer::Layer;

// #[server]
// pub async fn secure_action() -> Result<(), ServerFnError> {
//     // body runs only if middleware allows request
//     Ok(())
// }

// // Somewhere: implement middleware for this server fn type
// impl secure_action::ServerFn for SecureAction {
//     fn middlewares() -> MiddlewareSet<Req, Res> {
//         vec![
//             Arc::new(RequireLoginLayer::new()),
//             // Arc::new(RequireRoleLayer::admin()),
//         ]
//     }
// }
// ```

// **What this would achieve:**
// - The server fn endpoint is protected even if it is under `/api/*` with no Axum route grouping.
// - Each server fn can have different middleware (login-only vs admin-only vs rate limited, etc.).

// **Pros:**
// - Fine-grained, per-function policy.
// - Doesn’t rely on router structure.

// **Cons (why most Axum apps don’t use it):**
// - More complex; harder to see “what is protected” at a glance.
// - You still need to access session/auth data inside layers (extractors/state wiring).
// - In practice, router-level middleware is simpler and clearer for Axum.

// ---

// ### “How Leptos originally wanted it”
// Leptos’ core message is not “use MiddlewareSet”; it’s:

// 1) `#[server]` functions are **public HTTP endpoints**.
// 2) Therefore enforce **security on the server** (auth/roles/CSRF), not in the UI.

// Leptos provides `MiddlewareSet` as an *optional mechanism* to layer security per server fn. In Axum, it’s equally valid (and usually preferable) to enforce auth via Axum router middleware, which is what you did.

// ---

// ### One-liner summary
// - **You did:** Axum router groups + `login_required!` + per-fn role checks. (Simple, idiomatic)
// - **MiddlewareSet approach:** attach tower layers per server fn endpoint. (More granular, more complex)

// If you later want “admin middleware automatically,” the Axum-style version is usually: create `/api/admin/*` + an admin-check layer, rather than `MiddlewareSet`.
