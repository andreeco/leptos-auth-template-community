// Known limitation:
// If you change user roles/permissions on the server, existing sessions will keep their old roles until logout or session expiration.
// To force immediate session invalidation, include a version/timestamp in session_auth_hash() and bump it on permission/role changes.

use leptos::prelude::*;
use std::collections::HashSet;

pub use crate::auth::{Permission, Role};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UserSummary {
    pub id: u64,
    pub username: String,
    pub roles: HashSet<Role>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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

    pub fn has_perm(&self, perm: &str) -> bool {
        self.permissions
            .get()
            .contains(&Permission(perm.to_string()))
    }
}

#[server(AuthSnapshotFn)]
pub async fn auth_snapshot() -> Result<AuthSnapshot, ServerFnError> {
    use crate::auth::AuthSession;
    use axum::Extension;

    let Extension(auth): Extension<AuthSession> = leptos_axum::extract().await?;

    let user = auth.user.clone().map(|u| UserSummary {
        id: u.id,
        username: u.username,
        roles: u.roles,
    });

    let permissions = auth.user.clone().map(|u| u.permissions).unwrap_or_default();

    Ok(AuthSnapshot { user, permissions })
}
