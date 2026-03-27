use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(not(any(feature = "ssr", feature = "hydrate")))]
mod webauthn_rs_proto {
    pub mod attest {
        use serde::{Deserialize, Serialize};

        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct CreationChallengeResponse;
        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct RegisterPublicKeyCredential;
    }
    pub mod auth {
        use serde::{Deserialize, Serialize};

        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct RequestChallengeResponse;
        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct PublicKeyCredential;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebauthnCredentialRow {
    pub id: i64,
    pub name: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountProfileDto {
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
pub mod client {
    use super::*;
    use wasm_bindgen::JsCast;

    pub async fn create_credential(
        ccr: webauthn_rs_proto::attest::CreationChallengeResponse,
    ) -> Result<webauthn_rs_proto::attest::RegisterPublicKeyCredential, ServerFnError> {
        use web_sys::{window, CredentialCreationOptions, PublicKeyCredential};

        let opts: CredentialCreationOptions = ccr.into();
        let promise = window()
            .ok_or_else(|| ServerFnError::new("err_no_window"))?
            .navigator()
            .credentials()
            .create_with_options(&opts)
            .map_err(|_| ServerFnError::new("err_navigator_credentials_create_failed"))?;

        let js_cred: wasm_bindgen::JsValue = wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(|_| ServerFnError::new("err_create_credential_rejected"))?;

        let pkc: PublicKeyCredential = js_cred
            .dyn_into()
            .map_err(|_| ServerFnError::new("err_create_credential_invalid_response"))?;

        Ok(webauthn_rs_proto::attest::RegisterPublicKeyCredential::from(pkc))
    }

    pub async fn get_credential(
        rcr: webauthn_rs_proto::auth::RequestChallengeResponse,
    ) -> Result<webauthn_rs_proto::auth::PublicKeyCredential, ServerFnError> {
        use web_sys::{window, CredentialRequestOptions, PublicKeyCredential};

        let opts: CredentialRequestOptions = rcr.into();
        let promise = window()
            .ok_or_else(|| ServerFnError::new("err_no_window"))?
            .navigator()
            .credentials()
            .get_with_options(&opts)
            .map_err(|_| ServerFnError::new("err_navigator_credentials_get_failed"))?;

        let js_cred: wasm_bindgen::JsValue = wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(|_| ServerFnError::new("err_get_credential_rejected"))?;

        let pkc: PublicKeyCredential = js_cred
            .dyn_into()
            .map_err(|_| ServerFnError::new("err_get_credential_invalid_response"))?;

        Ok(webauthn_rs_proto::auth::PublicKeyCredential::from(pkc))
    }
}

#[cfg(not(all(feature = "hydrate", target_arch = "wasm32")))]
pub mod client {
    use super::*;

    pub async fn create_credential(
        _ccr: webauthn_rs_proto::attest::CreationChallengeResponse,
    ) -> Result<webauthn_rs_proto::attest::RegisterPublicKeyCredential, ServerFnError> {
        Err(ServerFnError::new("err_webauthn_requires_wasm"))
    }

    pub async fn get_credential(
        _rcr: webauthn_rs_proto::auth::RequestChallengeResponse,
    ) -> Result<webauthn_rs_proto::auth::PublicKeyCredential, ServerFnError> {
        Err(ServerFnError::new("err_webauthn_requires_wasm"))
    }
}

pub use client::{create_credential, get_credential};

#[cfg(feature = "ssr")]
mod ssr {
    use super::*;
    use crate::auth::AuthSession;
    use crate::entities::{users, webauthn_credentials};
    use crate::state::AppState;
    use axum::Extension;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use chrono::Utc;
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
    use tower_sessions::Session;
    use uuid::Uuid;
    use webauthn_rs::prelude::{
        AuthenticationResult, CredentialID, DiscoverableAuthentication, DiscoverableKey, Passkey,
        PasskeyRegistration,
    };

    const REG_STATE_KEY: &str = "webauthn_reg_state";
    const REG_NAME_KEY: &str = "webauthn_reg_name";
    const DISC_STATE_KEY: &str = "webauthn_disc_auth_state";

    fn b64u_encode(bytes: &[u8]) -> String {
        URL_SAFE_NO_PAD.encode(bytes)
    }

    fn b64u_decode(s: &str) -> Result<Vec<u8>, ServerFnError> {
        URL_SAFE_NO_PAD
            .decode(s.as_bytes())
            .map_err(|_| ServerFnError::new("err_base64url_decode_failed"))
    }

    fn model_to_passkey(m: &webauthn_credentials::Model) -> Result<Passkey, ServerFnError> {
        serde_json::from_str::<Passkey>(&m.passkey_json)
            .map_err(|_| ServerFnError::new("err_invalid_stored_passkey_json"))
    }

    async fn session() -> Result<Session, ServerFnError> {
        let Extension(sess): Extension<Session> = leptos_axum::extract().await?;
        Ok(sess)
    }

    async fn require_user() -> Result<crate::auth::User, ServerFnError> {
        let Extension(auth): Extension<AuthSession> = leptos_axum::extract().await?;
        auth.user
            .clone()
            .ok_or_else(|| ServerFnError::new("err_not_authenticated"))
    }

    pub async fn account_change_password(
        csrf: &str,
        current_password: String,
        new_password: String,
        new_password_confirm: String,
    ) -> Result<(), ServerFnError> {
        crate::csrf::require_csrf(csrf).await?;
        let user = require_user().await?;
        let app_state = expect_context::<AppState>();

        if new_password != new_password_confirm {
            return Err(ServerFnError::new("err_password_mismatch"));
        }

        let new_password = new_password.trim().to_string();
        if new_password.len() < 10 {
            return Err(ServerFnError::new("err_password_too_short"));
        }

        let Some(current) = users::Entity::find_by_id(user.id as i64)
            .one(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?
        else {
            return Err(ServerFnError::new("err_user_not_found"));
        };

        let ok = password_auth::verify_password(current_password, &current.password_hash).is_ok();
        if !ok {
            return Err(ServerFnError::new("err_current_password_incorrect"));
        }

        let new_hash = password_auth::generate_hash(&new_password);
        let mut am: users::ActiveModel = current.into();
        am.password_hash = Set(new_hash);
        am.password_reset_required = Set(false);
        am.updated_at = Set(Utc::now());
        am.update(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?;

        let _ = crate::csrf::rotate_csrf_token().await;
        Ok(())
    }

    pub async fn account_profile_get() -> Result<AccountProfileDto, ServerFnError> {
        let user = require_user().await?;
        let app_state = expect_context::<AppState>();

        let Some(current) = users::Entity::find_by_id(user.id as i64)
            .one(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?
        else {
            return Err(ServerFnError::new("err_user_not_found"));
        };

        Ok(AccountProfileDto {
            username: current.username,
            first_name: current.first_name,
            last_name: current.last_name,
            email: current.email,
        })
    }

    pub async fn account_profile_update(
        csrf: &str,
        username: String,
        first_name: String,
        last_name: String,
        email: String,
    ) -> Result<AccountProfileDto, ServerFnError> {
        crate::csrf::require_csrf(csrf).await?;
        let user = require_user().await?;
        let app_state = expect_context::<AppState>();

        let username = username.trim().to_string();
        let first_name = first_name.trim().to_string();
        let last_name = last_name.trim().to_string();
        let email = email.trim().to_lowercase();

        if username.is_empty() {
            return Err(ServerFnError::new("err_username_required"));
        }

        if email.is_empty() || !email.contains('@') {
            return Err(ServerFnError::new("err_invalid_email"));
        }

        let username_taken = users::Entity::find()
            .filter(users::Column::Username.eq(username.clone()))
            .filter(users::Column::Id.ne(user.id as i64))
            .one(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?
            .is_some();

        if username_taken {
            return Err(ServerFnError::new("err_username_exists"));
        }

        let email_taken = users::Entity::find()
            .filter(users::Column::Email.eq(email.clone()))
            .filter(users::Column::Id.ne(user.id as i64))
            .one(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?
            .is_some();

        if email_taken {
            return Err(ServerFnError::new("err_email_taken"));
        }

        let Some(current) = users::Entity::find_by_id(user.id as i64)
            .one(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?
        else {
            return Err(ServerFnError::new("err_user_not_found"));
        };

        let mut am: users::ActiveModel = current.into();
        am.username = Set(username.clone());
        am.first_name = Set(first_name.clone());
        am.last_name = Set(last_name.clone());
        am.email = Set(email.clone());
        am.updated_at = Set(Utc::now());
        am.update(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?;

        let _ = crate::csrf::rotate_csrf_token().await;

        Ok(AccountProfileDto {
            username,
            first_name,
            last_name,
            email,
        })
    }

    pub async fn account_webauthn_list() -> Result<Vec<WebauthnCredentialRow>, ServerFnError> {
        let user = require_user().await?;
        let app_state = expect_context::<AppState>();

        let rows = webauthn_credentials::Entity::find()
            .filter(webauthn_credentials::Column::UserId.eq(user.id as i64))
            .all(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?;

        Ok(rows
            .into_iter()
            .map(|m| WebauthnCredentialRow {
                id: m.id,
                name: m.name,
                created_at: m.created_at.to_string(),
            })
            .collect())
    }

    pub async fn account_webauthn_delete(csrf: &str, id: i64) -> Result<(), ServerFnError> {
        crate::csrf::require_csrf(csrf).await?;
        let user = require_user().await?;
        let app_state = expect_context::<AppState>();

        let Some(m) = webauthn_credentials::Entity::find_by_id(id)
            .one(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?
        else {
            return Ok(());
        };

        if m.user_id != user.id as i64 {
            return Err(ServerFnError::new("err_forbidden"));
        }

        webauthn_credentials::Entity::delete_by_id(id)
            .exec(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?;

        Ok(())
    }

    pub async fn account_webauthn_register_start(
        name: String,
    ) -> Result<webauthn_rs_proto::attest::CreationChallengeResponse, ServerFnError> {
        let user = require_user().await?;
        let app_state = expect_context::<AppState>();
        let sess = session().await?;

        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(ServerFnError::new("err_passkey_name_required"));
        }

        let existing = webauthn_credentials::Entity::find()
            .filter(webauthn_credentials::Column::UserId.eq(user.id as i64))
            .all(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?;

        let mut exclude: Vec<CredentialID> = Vec::new();
        for m in existing {
            let b = b64u_decode(&m.credential_id)?;
            exclude.push(b.into());
        }

        let Some(mut user_model) = users::Entity::find_by_id(user.id as i64)
            .one(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?
        else {
            return Err(ServerFnError::new("err_user_not_found"));
        };

        if user_model.webauthn_user_handle.is_none() {
            user_model.webauthn_user_handle = Some(Uuid::new_v4().to_string());
            let mut am: users::ActiveModel = user_model.clone().into();
            am.webauthn_user_handle = Set(user_model.webauthn_user_handle.clone());
            am.updated_at = Set(Utc::now());
            am.update(&app_state.db)
                .await
                .map_err(|_e| ServerFnError::new("err_internal"))?;
        }

        let user_handle_str = user_model
            .webauthn_user_handle
            .ok_or_else(|| ServerFnError::new("err_missing_webauthn_user_handle"))?;
        let user_handle = Uuid::parse_str(&user_handle_str)
            .map_err(|_| ServerFnError::new("err_invalid_webauthn_user_handle"))?;

        let (ccr, reg_state): (
            webauthn_rs_proto::attest::CreationChallengeResponse,
            PasskeyRegistration,
        ) = app_state
            .webauthn
            .start_passkey_registration(user_handle, &user.username, &user.username, Some(exclude))
            .map_err(|_e| ServerFnError::new("err_internal"))?;

        sess.insert(REG_STATE_KEY, reg_state)
            .await
            .map_err(|_| ServerFnError::new("err_session_error"))?;
        sess.insert(REG_NAME_KEY, name)
            .await
            .map_err(|_| ServerFnError::new("err_session_error"))?;

        Ok(ccr)
    }

    pub async fn account_webauthn_register_finish(
        csrf: &str,
        rpkc: webauthn_rs_proto::attest::RegisterPublicKeyCredential,
    ) -> Result<(), ServerFnError> {
        crate::csrf::require_csrf(csrf).await?;
        let user = require_user().await?;
        let app_state = expect_context::<AppState>();
        let sess = session().await?;

        let reg_state: PasskeyRegistration = sess
            .get(REG_STATE_KEY)
            .await
            .map_err(|_| ServerFnError::new("err_session_error"))?
            .ok_or_else(|| ServerFnError::new("err_missing_registration_state"))?;
        let _ = sess.remove::<PasskeyRegistration>(REG_STATE_KEY).await;

        let name: String = sess
            .get(REG_NAME_KEY)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "Passkey".to_string());
        let _ = sess.remove::<String>(REG_NAME_KEY).await;

        let passkey: Passkey = app_state
            .webauthn
            .finish_passkey_registration(&rpkc, &reg_state)
            .map_err(|_e| ServerFnError::new("err_internal"))?;

        let now = Utc::now();
        let passkey_json = serde_json::to_string(&passkey)
            .map_err(|_| ServerFnError::new("Passkey JSON serialize failed"))?;
        let cred_id_b64 = b64u_encode(passkey.cred_id().as_ref());

        webauthn_credentials::ActiveModel {
            user_id: Set(user.id as i64),
            credential_id: Set(cred_id_b64),
            passkey_json: Set(passkey_json),
            sign_count: Set(0),
            name: Set(name),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&app_state.db)
        .await
        .map_err(|_e| ServerFnError::new("err_internal"))?;

        let _ = crate::csrf::rotate_csrf_token().await;
        Ok(())
    }

    pub async fn webauthn_login_start(
    ) -> Result<webauthn_rs_proto::auth::RequestChallengeResponse, ServerFnError> {
        let app_state = expect_context::<AppState>();
        let sess = session().await?;

        let _ = sess
            .remove::<DiscoverableAuthentication>(DISC_STATE_KEY)
            .await;

        let (rcr, disc_state) = app_state
            .webauthn
            .start_discoverable_authentication()
            .map_err(|_e| ServerFnError::new("err_internal"))?;

        sess.insert(DISC_STATE_KEY, disc_state)
            .await
            .map_err(|_| ServerFnError::new("err_session_error"))?;

        let rcr = webauthn_rs_proto::auth::RequestChallengeResponse {
            public_key: rcr.public_key,
            mediation: None,
        };

        Ok(rcr)
    }

    pub async fn webauthn_login_finish(
        csrf: &str,
        pkc: webauthn_rs_proto::auth::PublicKeyCredential,
    ) -> Result<(), ServerFnError> {
        crate::csrf::require_csrf(csrf).await?;

        use axum_login::AuthnBackend;
        use http::StatusCode;
        use leptos_axum::ResponseOptions;

        let response = expect_context::<ResponseOptions>();
        let app_state = expect_context::<AppState>();
        let sess = session().await?;

        let disc_state: DiscoverableAuthentication = sess
            .get(DISC_STATE_KEY)
            .await
            .map_err(|_| ServerFnError::new("err_session_error"))?
            .ok_or_else(|| {
                response.set_status(StatusCode::BAD_REQUEST);
                ServerFnError::new("err_missing_discoverable_auth_state")
            })?;

        let _ = sess
            .remove::<DiscoverableAuthentication>(DISC_STATE_KEY)
            .await;

        let (user_handle_uuid, _cred_id) = app_state
            .webauthn
            .identify_discoverable_authentication(&pkc)
            .map_err(|_e| {
                response.set_status(StatusCode::UNAUTHORIZED);
                ServerFnError::new("err_internal")
            })?;

        let u = users::Entity::find()
            .filter(users::Column::WebauthnUserHandle.eq(Some(user_handle_uuid.to_string())))
            .one(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?
            .ok_or_else(|| {
                response.set_status(StatusCode::UNAUTHORIZED);
                ServerFnError::new("err_unknown_user_for_discoverable_credential")
            })?;

        let status_active = u.status.trim().eq_ignore_ascii_case("active");
        if !status_active || u.password_reset_required {
            response.set_status(StatusCode::UNAUTHORIZED);
            return Err(ServerFnError::new("error_invalid_credentials"));
        }

        let models = webauthn_credentials::Entity::find()
            .filter(webauthn_credentials::Column::UserId.eq(u.id))
            .all(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?;

        if models.is_empty() {
            response.set_status(StatusCode::UNAUTHORIZED);
            return Err(ServerFnError::new("err_no_passkeys_registered_for_user"));
        }

        let mut passkeys: Vec<Passkey> = Vec::with_capacity(models.len());
        for m in &models {
            passkeys.push(model_to_passkey(m)?);
        }
        let dks: Vec<DiscoverableKey> = passkeys.iter().map(DiscoverableKey::from).collect();

        let auth_res: AuthenticationResult = app_state
            .webauthn
            .finish_discoverable_authentication(&pkc, disc_state, &dks)
            .map_err(|_e| {
                response.set_status(StatusCode::UNAUTHORIZED);
                ServerFnError::new("err_internal")
            })?;

        let now = Utc::now();
        for m in models {
            let mut pk = model_to_passkey(&m)?;
            if let Some(true) = pk.update_credential(&auth_res) {
                let pk_json = serde_json::to_string(&pk)
                    .map_err(|_| ServerFnError::new("err_passkey_json_serialize_failed"))?;
                let mut am: webauthn_credentials::ActiveModel = m.into();
                am.passkey_json = Set(pk_json);
                am.sign_count = Set(auth_res.counter() as i64);
                am.updated_at = Set(now);
                am.update(&app_state.db)
                    .await
                    .map_err(|_e| ServerFnError::new("err_internal"))?;
            }
        }

        let Extension(mut auth): Extension<AuthSession> = leptos_axum::extract().await?;
        let Extension(session): Extension<Session> = leptos_axum::extract().await?;

        session
            .cycle_id()
            .await
            .map_err(|_| ServerFnError::new("err_session_error"))?;

        let uid_u64 = u.id as u64;
        let user = auth
            .backend
            .get_user(&uid_u64)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?
            .ok_or_else(|| ServerFnError::new("err_user_not_found"))?;

        auth.login(&user)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?;

        let _ = crate::csrf::rotate_csrf_token().await;
        Ok(())
    }
}

#[server(prefix = "/api/secure")]
pub async fn account_change_password(
    csrf: String,
    current_password: String,
    new_password: String,
    new_password_confirm: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        ssr::account_change_password(&csrf, current_password, new_password, new_password_confirm).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (csrf, current_password, new_password, new_password_confirm);
        Err(ServerFnError::new("err_server_only"))
    }
}

#[server(prefix = "/api/secure")]
pub async fn account_profile_get() -> Result<AccountProfileDto, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        ssr::account_profile_get().await
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::new("err_server_only"))
    }
}

#[server(prefix = "/api/secure")]
pub async fn account_profile_update(
    csrf: String,
    username: String,
    first_name: String,
    last_name: String,
    email: String,
) -> Result<AccountProfileDto, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        ssr::account_profile_update(&csrf, username, first_name, last_name, email).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (csrf, username, first_name, last_name, email);
        Err(ServerFnError::new("err_server_only"))
    }
}

#[server(prefix = "/api/secure")]
pub async fn account_webauthn_list() -> Result<Vec<WebauthnCredentialRow>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        ssr::account_webauthn_list().await
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::new("err_server_only"))
    }
}

#[server(prefix = "/api/secure")]
pub async fn account_webauthn_delete(csrf: String, id: i64) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        ssr::account_webauthn_delete(&csrf, id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (csrf, id);
        Err(ServerFnError::new("err_server_only"))
    }
}

#[server(prefix = "/api/secure")]
pub async fn account_webauthn_register_start(
    name: String,
) -> Result<webauthn_rs_proto::attest::CreationChallengeResponse, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        ssr::account_webauthn_register_start(name).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = name;
        Err(ServerFnError::new("err_server_only"))
    }
}

#[server(prefix = "/api/secure")]
pub async fn account_webauthn_register_finish(
    csrf: String,
    rpkc: webauthn_rs_proto::attest::RegisterPublicKeyCredential,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        ssr::account_webauthn_register_finish(&csrf, rpkc).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (csrf, rpkc);
        Err(ServerFnError::new("err_server_only"))
    }
}

#[server(prefix = "/api", endpoint = "webauthn_login_start")]
pub async fn webauthn_login_start(
) -> Result<webauthn_rs_proto::auth::RequestChallengeResponse, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        ssr::webauthn_login_start().await
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::new("err_server_only"))
    }
}

#[server(prefix = "/api", endpoint = "webauthn_login_finish")]
pub async fn webauthn_login_finish(
    csrf: String,
    pkc: webauthn_rs_proto::auth::PublicKeyCredential,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        ssr::webauthn_login_finish(&csrf, pkc).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (csrf, pkc);
        Err(ServerFnError::new("err_server_only"))
    }
}
