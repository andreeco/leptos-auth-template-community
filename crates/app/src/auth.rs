use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    User,
    Admin,
    Staff,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Permission(pub String);

#[cfg(feature = "ssr")]
use axum_login::{AuthUser, AuthnBackend, UserId};

#[cfg(feature = "ssr")]
use password_auth::verify_password;

#[cfg(feature = "ssr")]
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect};

#[cfg(feature = "ssr")]
use std::collections::HashSet;

#[cfg(feature = "ssr")]
#[derive(Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub password_hash: String,
    pub status: String,
    pub password_reset_required: bool,
    pub session_auth_hash: String,
    pub roles: HashSet<Role>,
    pub permissions: HashSet<Permission>,
}

#[cfg(feature = "ssr")]
impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .field("username", &self.username)
            .field("password_hash", &"[redacted]")
            .field("status", &self.status)
            .field("password_reset_required", &self.password_reset_required)
            .field("session_auth_hash", &"[derived]")
            .field("roles", &self.roles)
            .field("permissions", &self.permissions)
            .finish()
    }
}

#[cfg(feature = "ssr")]
impl AuthUser for User {
    type Id = u64;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.session_auth_hash.as_bytes()
    }
}

#[cfg(feature = "ssr")]
#[derive(Debug, Clone, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

#[cfg(feature = "ssr")]
#[derive(Clone)]
pub struct Backend {
    db: DatabaseConnection,
}

#[cfg(feature = "ssr")]
impl Backend {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    fn derive_session_auth_hash(
        password_hash: &str,
        status: &str,
        password_reset_required: bool,
        roles: &HashSet<Role>,
        permissions: &HashSet<Permission>,
    ) -> String {
        let normalized_status = status.trim().to_lowercase();

        let mut role_names = roles
            .iter()
            .map(|role| match role {
                Role::User => "user",
                Role::Admin => "admin",
                Role::Staff => "staff",
            })
            .collect::<Vec<_>>();
        role_names.sort_unstable();

        let mut permission_names = permissions
            .iter()
            .map(|perm| perm.0.as_str())
            .collect::<Vec<_>>();
        permission_names.sort_unstable();

        format!(
            "{}|status={}|password_reset_required={}|roles={}|permissions={}",
            password_hash,
            normalized_status,
            password_reset_required,
            role_names.join(","),
            permission_names.join(",")
        )
    }

    async fn build_user(&self, u: crate::entities::users::Model) -> Result<User, BackendError> {
        use crate::entities::{permissions, role_permissions, roles, user_roles};

        // 1) Get role ids for the user.
        let role_ids: Vec<i64> = user_roles::Entity::find()
            .filter(user_roles::Column::UserId.eq(u.id))
            .select_only()
            .column(user_roles::Column::RoleId)
            .into_tuple()
            .all(&self.db)
            .await
            .map_err(|e| BackendError::Internal(e.to_string()))?;

        // 2) Resolve role names.
        let role_names: Vec<String> = if role_ids.is_empty() {
            Vec::new()
        } else {
            roles::Entity::find()
                .filter(roles::Column::Id.is_in(role_ids.clone()))
                .select_only()
                .column(roles::Column::Name)
                .into_tuple()
                .all(&self.db)
                .await
                .map_err(|e| BackendError::Internal(e.to_string()))?
        };

        let mut role_set: HashSet<Role> = HashSet::new();
        for role_name in role_names {
            if role_name.eq_ignore_ascii_case("admin") {
                role_set.insert(Role::Admin);
            } else if role_name.eq_ignore_ascii_case("staff") {
                role_set.insert(Role::Staff);
            } else {
                role_set.insert(Role::User);
            }
        }
        // Practical default if DB roles are not assigned yet.
        if role_set.is_empty() {
            role_set.insert(Role::User);
        }

        // 3) Get permission ids from the user's roles.
        let perm_ids: Vec<i64> = if role_ids.is_empty() {
            Vec::new()
        } else {
            role_permissions::Entity::find()
                .filter(role_permissions::Column::RoleId.is_in(role_ids))
                .select_only()
                .column(role_permissions::Column::PermissionId)
                .into_tuple()
                .all(&self.db)
                .await
                .map_err(|e| BackendError::Internal(e.to_string()))?
        };

        // 4) Resolve permission names.
        let perm_names: Vec<String> = if perm_ids.is_empty() {
            Vec::new()
        } else {
            permissions::Entity::find()
                .filter(permissions::Column::Id.is_in(perm_ids))
                .select_only()
                .column(permissions::Column::Name)
                .into_tuple()
                .all(&self.db)
                .await
                .map_err(|e| BackendError::Internal(e.to_string()))?
        };

        let permission_set = perm_names
            .into_iter()
            .map(Permission)
            .collect::<HashSet<_>>();

        let session_auth_hash = Self::derive_session_auth_hash(
            &u.password_hash,
            &u.status,
            u.password_reset_required,
            &role_set,
            &permission_set,
        );

        Ok(User {
            id: u.id as u64,
            username: u.username,
            password_hash: u.password_hash,
            status: u.status,
            password_reset_required: u.password_reset_required,
            session_auth_hash,
            roles: role_set,
            permissions: permission_set,
        })
    }
}

#[cfg(feature = "ssr")]
#[derive(thiserror::Error, Debug)]
pub enum BackendError {
    #[error("internal error: {0}")]
    Internal(String),
}

#[cfg(feature = "ssr")]
impl AuthnBackend for Backend {
    type User = User;
    type Credentials = Credentials;
    type Error = BackendError;

    fn authenticate(
        &self,
        creds: Credentials,
    ) -> impl std::future::Future<Output = Result<Option<User>, BackendError>> + Send {
        let this = self.clone();

        async move {
            use crate::entities::users;

            let db_user = users::Entity::find()
                .filter(users::Column::Username.eq(creds.username))
                .one(&this.db)
                .await
                .map_err(|e| BackendError::Internal(e.to_string()))?;

            let Some(db_user) = db_user else {
                return Ok(None);
            };

            let hash = db_user.password_hash.clone();
            let ok = tokio::task::spawn_blocking(move || verify_password(creds.password, &hash).is_ok())
                .await
                .map_err(|e| BackendError::Internal(e.to_string()))?;

            if !ok {
                return Ok(None);
            }

            let status_active = db_user.status.trim().eq_ignore_ascii_case("active");
            if !status_active {
                return Ok(None);
            }

            let user = this.build_user(db_user).await?;
            Ok(Some(user))
        }
    }

    fn get_user(
        &self,
        user_id: &UserId<Self>,
    ) -> impl std::future::Future<Output = Result<Option<User>, BackendError>> + Send {
        let this = self.clone();
        let uid = *user_id;

        async move {
            use crate::entities::users;

            let id_i64 = i64::try_from(uid)
                .map_err(|_| BackendError::Internal("user id conversion overflow".into()))?;

            let db_user = users::Entity::find_by_id(id_i64)
                .one(&this.db)
                .await
                .map_err(|e| BackendError::Internal(e.to_string()))?;

            let Some(db_user) = db_user else {
                return Ok(None);
            };

            let user = this.build_user(db_user).await?;
            Ok(Some(user))
        }
    }
}

#[cfg(feature = "ssr")]
pub type AuthSession = axum_login::AuthSession<Backend>;
