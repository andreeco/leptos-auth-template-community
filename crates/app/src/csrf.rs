use leptos::prelude::*;
use serde::{Deserialize, Serialize};

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
    use std::fmt;
    use subtle::ConstantTimeEq;
    use tower_sessions::Session;

    const CSRF_KEY: &str = "csrf_token";

    #[derive(Debug)]
    struct CsrfError(&'static str);

    impl fmt::Display for CsrfError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.0)
        }
    }

    impl std::error::Error for CsrfError {}

    fn err(msg: &'static str) -> ServerFnError {
        ServerFnError::from(CsrfError(msg))
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
            .map_err(|_| err("CSRF session error"))?;
        Ok(CsrfToken { token })
    }

    pub async fn require(submitted: &str) -> Result<(), ServerFnError> {
        let Extension(session): Extension<Session> = leptos_axum::extract().await?;
        let ok = verify_csrf_token(&session, submitted)
            .await
            .map_err(|_| err("CSRF session error"))?;

        if !ok {
            return Err(err("CSRF validation failed"));
        }

        Ok(())
    }

    pub async fn rotate() -> Result<CsrfToken, ServerFnError> {
        let Extension(session): Extension<Session> = leptos_axum::extract().await?;
        let token = set_new_csrf_token(&session)
            .await
            .map_err(|_| err("CSRF session error"))?;
        Ok(CsrfToken { token })
    }

    pub async fn get_or_create_for_session(session: &Session) -> Result<CsrfToken, ServerFnError> {
        let token = get_or_create_csrf_token(session)
            .await
            .map_err(|_| err("CSRF session error"))?;
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
