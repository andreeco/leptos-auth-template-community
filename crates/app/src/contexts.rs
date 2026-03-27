use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub use crate::features::auth::{Permission, Role};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserSummary {
    pub id: u64,
    pub username: String,
    pub roles: HashSet<Role>,
    pub password_reset_required: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthSnapshot {
    pub user: Option<UserSummary>,
    pub permissions: HashSet<Permission>,
}

#[derive(Copy, Clone)]
pub struct AuthState {
    pub ready: ReadSignal<bool>,
    pub set_ready: WriteSignal<bool>,
    pub user: ReadSignal<Option<UserSummary>>,
    pub set_user: WriteSignal<Option<UserSummary>>,
    pub permissions: ReadSignal<HashSet<Permission>>,
    pub set_permissions: WriteSignal<HashSet<Permission>>,
}

impl AuthState {
    pub fn logged_in(&self) -> bool {
        self.user.get().is_some()
    }

    pub fn requires_password_reset(&self) -> bool {
        self.user
            .get()
            .map(|u| u.password_reset_required)
            .unwrap_or(false)
    }

    pub fn username(&self) -> Option<String> {
        self.user.get().map(|u| u.username)
    }

    pub fn has_role(&self, role: Role) -> bool {
        self.user
            .get()
            .map(|u| u.roles.contains(&role))
            .unwrap_or(false)
    }

    pub fn is_admin(&self) -> bool {
        self.has_role(Role::Admin)
    }

    pub fn is_staff(&self) -> bool {
        self.has_role(Role::Staff)
    }

    pub fn has_perm(&self, perm: &str) -> bool {
        self.permissions
            .get()
            .contains(&Permission(perm.to_string()))
    }
}

#[server(AuthSnapshotFn)]
pub async fn auth_snapshot() -> Result<AuthSnapshot, ServerFnError> {
    use crate::features::auth::AuthSession;
    use axum::Extension;

    let Extension(auth): Extension<AuthSession> = leptos_axum::extract().await?;

    let user = auth.user.clone().map(|u| UserSummary {
        id: u.id,
        username: u.username,
        roles: u.roles,
        password_reset_required: u.password_reset_required,
    });

    let permissions = auth.user.clone().map(|u| u.permissions).unwrap_or_default();

    Ok(AuthSnapshot { user, permissions })
}

#[component]
pub fn ApplySsrAuthSnapshot() -> impl IntoView {
    let auth = expect_context::<AuthState>();
    let snap = use_context::<AuthSnapshot>();
    let applied = RwSignal::new(false);

    Effect::new(move |_| {
        if applied.get() {
            return;
        }
        let Some(AuthSnapshot { user, permissions }) = snap.clone() else {
            return;
        };

        auth.set_user.set(user);
        auth.set_permissions.set(permissions);
        auth.set_ready.set(true);
        applied.set(true);
    });

    ()
}

#[component]
pub fn EnsureAuthSnapshot() -> impl IntoView {
    let auth = expect_context::<AuthState>();
    let snap = use_context::<AuthSnapshot>();
    let snap_for_trigger = snap.clone();
    let snap_for_effect = snap.clone();

    let res = Resource::new(
        move || snap_for_trigger.is_none() && !auth.ready.get(),
        move |need| async move {
            if need {
                Some(auth_snapshot().await)
            } else {
                None
            }
        },
    );

    Effect::new(move |_| {
        if snap_for_effect.is_some() {
            return;
        }

        if let Some(Some(Ok(AuthSnapshot { user, permissions }))) = res.get() {
            auth.set_user.set(user);
            auth.set_permissions.set(permissions);
            auth.set_ready.set(true);
        }
    });

    ()
}

#[component]
pub fn EnsureCsrfToken() -> impl IntoView {
    let ctx = expect_context::<CsrfContext>().0;
    let refresh = expect_context::<RwSignal<()>>();
    let ssr_tok = use_context::<CsrfToken>();

    let applied_ssr = RwSignal::new(false);

    Effect::new(move |_| {
        if applied_ssr.get() {
            return;
        }

        if let Some(tok) = ssr_tok.clone() {
            if ctx.get().is_none() && !tok.token.is_empty() {
                ctx.set(Some(tok.token));
            }
        }

        applied_ssr.set(true);
    });

    let first_pass = RwSignal::new(true);
    let res = LocalResource::new(move || {
        refresh.get();

        let skip_first_fetch = first_pass.get_untracked() && ctx.get_untracked().is_some();
        first_pass.set(false);

        async move {
            if skip_first_fetch {
                None
            } else {
                Some(crate::contexts::get_csrf_token().await)
            }
        }
    });

    Effect::new(move |_| {
        if let Some(Some(Ok(tok))) = res.get() {
            ctx.set(Some(tok.token));
        }
    });

    ()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CsrfToken {
    pub token: String,
}

#[derive(Clone, Copy)]
pub struct CsrfContext(pub RwSignal<Option<String>>);

pub fn provide_csrf_context(csrf: &CsrfToken) {
    let csrf_sig = RwSignal::new((!csrf.token.is_empty()).then_some(csrf.token.clone()));
    provide_context(CsrfContext(csrf_sig));
}

#[cfg(feature = "ssr")]
mod ssr {
    use super::*;
    use axum::Extension;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use rand::rngs::OsRng;
    use rand::RngCore;
    use subtle::ConstantTimeEq;
    use tower_sessions::Session;

    const CSRF_KEY: &str = "csrf_token";

    fn err(msg: &'static str) -> ServerFnError {
        ServerFnError::new(msg)
    }

    fn new_token() -> String {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    }

    async fn get_or_create_csrf_token(
        session: &Session,
    ) -> Result<String, tower_sessions::session::Error> {
        if let Some(token) = session.get::<String>(CSRF_KEY).await? {
            return Ok(token);
        }

        let token = new_token();
        session.insert(CSRF_KEY, token.clone()).await?;
        Ok(token)
    }

    async fn set_new_csrf_token(
        session: &Session,
    ) -> Result<String, tower_sessions::session::Error> {
        let token = new_token();
        session.insert(CSRF_KEY, token.clone()).await?;
        Ok(token)
    }

    async fn verify_csrf_token(
        session: &Session,
        submitted: &str,
    ) -> Result<bool, tower_sessions::session::Error> {
        let Some(expected) = session.get::<String>(CSRF_KEY).await? else {
            return Ok(false);
        };

        Ok(expected.as_bytes().ct_eq(submitted.as_bytes()).into())
    }

    pub async fn get_token() -> Result<CsrfToken, ServerFnError> {
        let Extension(session): Extension<Session> = leptos_axum::extract().await?;
        let token = get_or_create_csrf_token(&session)
            .await
            .map_err(|_| err("err_session"))?;
        Ok(CsrfToken { token })
    }

    pub async fn require(submitted: &str) -> Result<(), ServerFnError> {
        let Extension(session): Extension<Session> = leptos_axum::extract().await?;
        let ok = verify_csrf_token(&session, submitted)
            .await
            .map_err(|_| err("err_session"))?;

        if !ok {
            return Err(err("err_missing_csrf"));
        }

        Ok(())
    }

    pub async fn rotate() -> Result<CsrfToken, ServerFnError> {
        let Extension(session): Extension<Session> = leptos_axum::extract().await?;
        let token = set_new_csrf_token(&session)
            .await
            .map_err(|_| err("err_session"))?;
        Ok(CsrfToken { token })
    }

    pub async fn get_or_create_for_session(session: &Session) -> Result<CsrfToken, ServerFnError> {
        let token = get_or_create_csrf_token(session)
            .await
            .map_err(|_| err("err_session"))?;
        Ok(CsrfToken { token })
    }
}

#[server(GetCsrfToken)]
pub async fn get_csrf_token() -> Result<CsrfToken, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        ssr::get_token().await
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::ServerError(
            "get_csrf_token can only run on the server".into(),
        ))
    }
}

#[server(RotateCsrfToken)]
pub async fn rotate_csrf_token() -> Result<CsrfToken, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        ssr::rotate().await
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::ServerError(
            "rotate_csrf_token can only run on the server".into(),
        ))
    }
}

#[cfg(feature = "ssr")]
pub async fn require_csrf(submitted: &str) -> Result<(), ServerFnError> {
    ssr::require(submitted).await
}

#[cfg(feature = "ssr")]
pub async fn csrf_for_ssr(session: &tower_sessions::Session) -> Result<CsrfToken, ServerFnError> {
    ssr::get_or_create_for_session(session).await
}
