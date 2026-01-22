#![cfg(feature = "ssr")]

use axum_login::{AuthUser, AuthnBackend, UserId};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

impl AuthUser for User {
    type Id = u64;
    fn id(&self) -> Self::Id {
        self.id
    }
    fn session_auth_hash(&self) -> &[u8] {
        self.password.as_bytes()
    }
}

#[derive(Debug, Error)]
pub enum BackendError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Clone)]
pub struct Backend {
    users: Arc<Mutex<HashMap<String, User>>>,
}

impl Backend {
    pub fn new() -> Self {
        // Example in-memory user
        let mut map = HashMap::new();
        map.insert(
            "user".into(),
            User {
                id: 1,
                username: "user".into(),
                password: "password".into(),
            },
        );
        Self {
            users: Arc::new(Mutex::new(map)),
        }
    }
}

impl AuthnBackend for Backend {
    type User = User;
    type Credentials = Credentials;
    type Error = BackendError;

    fn authenticate(
        &self,
        creds: Credentials,
    ) -> impl std::future::Future<Output = Result<Option<User>, BackendError>> + Send {
        let users = self.users.clone();
        async move {
            let guard = users.lock().await;
            Ok(guard
                .get(&creds.username)
                .filter(|u| u.password == creds.password)
                .cloned())
        }
    }

    fn get_user(
        &self,
        user_id: &UserId<Self>,
    ) -> impl std::future::Future<Output = Result<Option<User>, BackendError>> + Send {
        let users = self.users.clone();
        let id = *user_id;
        async move {
            let guard = users.lock().await;
            Ok(guard.values().find(|u| u.id == id).cloned())
        }
    }
}

pub type AuthSession = axum_login::AuthSession<Backend>;

#[cfg(feature = "ssr")]
pub async fn auth() -> Result<AuthSession, leptos::server_fn::ServerFnError> {
    let auth = leptos_axum::extract().await?;
    Ok(auth)
}
