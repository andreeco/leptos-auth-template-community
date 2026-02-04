use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    User,
    Admin,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Permission(pub String);

#[cfg(feature = "ssr")]
use axum_login::{AuthUser, AuthnBackend, UserId};

#[cfg(feature = "ssr")]
use password_auth::{generate_hash, verify_password};

#[cfg(feature = "ssr")]
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

#[cfg(feature = "ssr")]
#[derive(Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub password_hash: String,
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
        self.password_hash.as_bytes()
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
    by_username: Arc<tokio::sync::RwLock<HashMap<String, User>>>,
    by_id: Arc<tokio::sync::RwLock<HashMap<u64, User>>>,
}

#[cfg(feature = "ssr")]
impl Backend {
    pub fn new() -> Self {
        let user = User {
            id: 1,
            username: "user".into(),
            password_hash: generate_hash("password"),
            roles: HashSet::from([Role::User]),
            permissions: HashSet::new(),
        };

        let admin = User {
            id: 2,
            username: "admin".into(),
            password_hash: generate_hash("password"),
            roles: HashSet::from([Role::Admin]),
            permissions: HashSet::from([Permission("admin.read".into())]),
        };

        let mut by_username = HashMap::new();
        by_username.insert(user.username.clone(), user.clone());
        by_username.insert(admin.username.clone(), admin.clone());

        let mut by_id = HashMap::new();
        by_id.insert(user.id, user);
        by_id.insert(admin.id, admin);

        Self {
            by_username: Arc::new(tokio::sync::RwLock::new(by_username)),
            by_id: Arc::new(tokio::sync::RwLock::new(by_id)),
        }
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
        let by_username = Arc::clone(&self.by_username);

        async move {
            let (user, hash) = {
                let guard = by_username.read().await;
                let Some(u) = guard.get(&creds.username) else {
                    return Ok(None);
                };
                (u.clone(), u.password_hash.clone())
            };

            let ok =
                tokio::task::spawn_blocking(move || verify_password(creds.password, &hash).is_ok())
                    .await
                    .map_err(|e| BackendError::Internal(e.to_string()))?;

            Ok(ok.then_some(user))
        }
    }

    fn get_user(
        &self,
        user_id: &UserId<Self>,
    ) -> impl std::future::Future<Output = Result<Option<User>, BackendError>> + Send {
        let by_id = Arc::clone(&self.by_id);
        let id = *user_id;

        async move {
            let guard = by_id.read().await;
            Ok(guard.get(&id).cloned())
        }
    }
}

#[cfg(feature = "ssr")]
pub type AuthSession = axum_login::AuthSession<Backend>;
