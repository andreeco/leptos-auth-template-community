// Session invalidation notes:
// password hash, account status, and password-reset-required are included in session auth hashing.
// Role/permission changes can still remain active for existing sessions unless you also include an authz version/hash
// in session_auth_hash() and bump it whenever role/permission assignments change.

use leptos::prelude::*;
use std::collections::HashSet;

pub use crate::auth::{Permission, Role};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UserSummary {
    pub id: u64,
    pub username: String,
    pub roles: HashSet<Role>,
    pub password_reset_required: bool,
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
    use crate::auth::AuthSession;
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
