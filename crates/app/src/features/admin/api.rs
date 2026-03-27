use leptos::prelude::*;

use super::types::AdminUserRow;

#[cfg(feature = "ssr")]
mod ssr {
    use super::AdminUserRow;
    use crate::features::auth::{AuthSession, Role};
    use crate::entities::{roles, user_roles, users};
    use crate::state::AppState;
    use axum::Extension;
    use chrono::Utc;
    use leptos::prelude::{expect_context, ServerFnError};
    use sea_orm::{
        ActiveModelTrait, ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder,
        QuerySelect, Set,
    };
    use std::collections::BTreeSet;

    async fn require_admin() -> Result<crate::features::auth::User, ServerFnError> {
        let Extension(auth): Extension<AuthSession> = leptos_axum::extract().await?;
        let Some(user) = auth.user.clone() else {
            return Err(ServerFnError::new("err_not_authenticated"));
        };
        if !user.roles.contains(&Role::Admin) {
            return Err(ServerFnError::new("err_forbidden"));
        }
        Ok(user)
    }

    fn normalize_status(input: &str) -> Option<&'static str> {
        let s = input.trim().to_lowercase();
        match s.as_str() {
            "active" => Some("active"),
            "disabled" => Some("disabled"),
            _ => None,
        }
    }

    fn parse_roles_csv(input: &str) -> Result<Vec<String>, ServerFnError> {
        let allowed = ["user", "admin", "staff"];

        let mut out = BTreeSet::<String>::new();
        for raw in input.split(',') {
            let role = raw.trim().to_lowercase();
            if role.is_empty() {
                continue;
            }
            if !allowed.contains(&role.as_str()) {
                return Err(ServerFnError::new("err_invalid_role"));
            }
            out.insert(role);
        }

        if out.is_empty() {
            return Err(ServerFnError::new("err_roles_required"));
        }

        Ok(out.into_iter().collect())
    }

    async fn get_or_create_role_id(
        db: &sea_orm::DatabaseConnection,
        role_name: &str,
    ) -> Result<i64, ServerFnError> {
        if let Some(r) = roles::Entity::find()
            .filter(roles::Column::Name.eq(role_name))
            .one(db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?
        {
            return Ok(r.id);
        }

        let created = roles::ActiveModel {
            name: Set(role_name.to_string()),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|_e| ServerFnError::new("err_internal"))?;

        Ok(created.id)
    }

    async fn set_roles_for_user(
        db: &sea_orm::DatabaseConnection,
        user_id: i64,
        role_names: &[String],
    ) -> Result<(), ServerFnError> {
        user_roles::Entity::delete_many()
            .filter(user_roles::Column::UserId.eq(user_id))
            .exec(db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?;

        for role_name in role_names {
            let role_id = get_or_create_role_id(db, role_name).await?;
            user_roles::ActiveModel {
                user_id: Set(user_id),
                role_id: Set(role_id),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?;
        }

        Ok(())
    }

    pub async fn users_list() -> Result<Vec<AdminUserRow>, ServerFnError> {
        let _admin = require_admin().await?;
        let app_state = expect_context::<AppState>();

        let models = users::Entity::find()
            .order_by_asc(users::Column::Id)
            .all(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?;

        let mut out = Vec::with_capacity(models.len());

        for u in models {
            let role_ids: Vec<i64> = user_roles::Entity::find()
                .filter(user_roles::Column::UserId.eq(u.id))
                .select_only()
                .column(user_roles::Column::RoleId)
                .into_tuple()
                .all(&app_state.db)
                .await
                .map_err(|_e| ServerFnError::new("err_internal"))?;

            let mut role_names: Vec<String> = if role_ids.is_empty() {
                Vec::new()
            } else {
                roles::Entity::find()
                    .filter(roles::Column::Id.is_in(role_ids))
                    .select_only()
                    .column(roles::Column::Name)
                    .into_tuple()
                    .all(&app_state.db)
                    .await
                    .map_err(|_e| ServerFnError::new("err_internal"))?
            };

            role_names.sort();

            let is_admin = role_names.iter().any(|r| r.eq_ignore_ascii_case("admin"));

            let status = if u.status.trim().is_empty() {
                "active".to_string()
            } else {
                u.status.clone()
            };

            let created_at = u.created_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
            let updated_at = u.updated_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();

            out.push(AdminUserRow {
                id: u.id,
                username: u.username,
                first_name: u.first_name,
                last_name: u.last_name,
                email: u.email,
                status,
                roles: role_names,
                is_admin,
                password_reset_required: u.password_reset_required,
                created_at,
                updated_at,
            });
        }

        Ok(out)
    }

    pub async fn users_search(query: String) -> Result<Vec<AdminUserRow>, ServerFnError> {
        let _admin = require_admin().await?;
        let app_state = expect_context::<AppState>();

        let q = query.trim().to_lowercase();
        if q.len() < 2 {
            return Ok(Vec::new());
        }

        let models = users::Entity::find()
            .filter(
                Condition::any()
                    .add(users::Column::Username.contains(q.clone()))
                    .add(users::Column::Email.contains(q.clone())),
            )
            .order_by_asc(users::Column::Id)
            .all(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?;

        let mut out = Vec::with_capacity(models.len());

        for u in models {
            let role_ids: Vec<i64> = user_roles::Entity::find()
                .filter(user_roles::Column::UserId.eq(u.id))
                .select_only()
                .column(user_roles::Column::RoleId)
                .into_tuple()
                .all(&app_state.db)
                .await
                .map_err(|_e| ServerFnError::new("err_internal"))?;

            let mut role_names: Vec<String> = if role_ids.is_empty() {
                Vec::new()
            } else {
                roles::Entity::find()
                    .filter(roles::Column::Id.is_in(role_ids))
                    .select_only()
                    .column(roles::Column::Name)
                    .into_tuple()
                    .all(&app_state.db)
                    .await
                    .map_err(|_e| ServerFnError::new("err_internal"))?
            };

            role_names.sort();

            let is_admin = role_names.iter().any(|r| r.eq_ignore_ascii_case("admin"));

            let status = if u.status.trim().is_empty() {
                "active".to_string()
            } else {
                u.status.clone()
            };

            let created_at = u.created_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
            let updated_at = u.updated_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();

            out.push(AdminUserRow {
                id: u.id,
                username: u.username,
                first_name: u.first_name,
                last_name: u.last_name,
                email: u.email,
                status,
                roles: role_names,
                is_admin,
                password_reset_required: u.password_reset_required,
                created_at,
                updated_at,
            });
        }

        Ok(out)
    }

    pub async fn users_create(
        username: String,
        first_name: String,
        last_name: String,
        email: String,
        password: String,
        status: String,
        roles_csv: String,
        reset_required: bool,
    ) -> Result<(), ServerFnError> {
        let _admin = require_admin().await?;
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
        if password.len() < 10 {
            return Err(ServerFnError::new("err_password_too_short"));
        }

        let status = normalize_status(&status).ok_or_else(|| ServerFnError::new("err_invalid_status"))?;
        let roles = parse_roles_csv(&roles_csv)?;

        let username_exists = users::Entity::find()
            .filter(users::Column::Username.eq(username.clone()))
            .one(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?
            .is_some();

        if username_exists {
            return Err(ServerFnError::new("err_username_exists"));
        }

        let email_taken = users::Entity::find()
            .filter(users::Column::Email.eq(email.clone()))
            .one(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?
            .is_some();

        if email_taken {
            return Err(ServerFnError::new("err_email_taken"));
        }

        let now = Utc::now();
        let hash = password_auth::generate_hash(&password);
        let created = users::ActiveModel {
            username: Set(username),
            first_name: Set(first_name),
            last_name: Set(last_name),
            email: Set(email),
            password_hash: Set(hash),
            status: Set(status.to_string()),
            password_reset_required: Set(reset_required),
            webauthn_user_handle: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&app_state.db)
        .await
        .map_err(|_e| ServerFnError::new("err_internal"))?;

        set_roles_for_user(&app_state.db, created.id, &roles).await?;
        Ok(())
    }

    async fn users_update_internal(
        id: i64,
        username: String,
        first_name: String,
        last_name: String,
        email: String,
        status: String,
        roles_csv: String,
        reset_required: bool,
        new_password: Option<String>,
    ) -> Result<(), ServerFnError> {
        let _admin = require_admin().await?;
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

        let status = normalize_status(&status).ok_or_else(|| ServerFnError::new("err_invalid_status"))?;
        let roles = parse_roles_csv(&roles_csv)?;

        let Some(existing) = users::Entity::find_by_id(id)
            .one(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?
        else {
            return Err(ServerFnError::new("err_user_not_found"));
        };

        let name_taken = users::Entity::find()
            .filter(users::Column::Username.eq(username.clone()))
            .filter(users::Column::Id.ne(id))
            .one(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?
            .is_some();

        if name_taken {
            return Err(ServerFnError::new("err_username_exists"));
        }

        let email_taken = users::Entity::find()
            .filter(users::Column::Email.eq(email.clone()))
            .filter(users::Column::Id.ne(id))
            .one(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?
            .is_some();

        if email_taken {
            return Err(ServerFnError::new("err_email_taken"));
        }

        let password_to_apply = new_password
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty());

        if let Some(ref p) = password_to_apply {
            if p.len() < 10 {
                return Err(ServerFnError::new("err_password_too_short"));
            }
        }

        let mut am: users::ActiveModel = existing.into();
        am.username = Set(username);
        am.first_name = Set(first_name);
        am.last_name = Set(last_name);
        am.email = Set(email);
        am.status = Set(status.to_string());
        am.password_reset_required = Set(reset_required);

        if let Some(p) = password_to_apply {
            am.password_hash = Set(password_auth::generate_hash(&p));
        }

        am.updated_at = Set(Utc::now());
        am.update(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?;

        set_roles_for_user(&app_state.db, id, &roles).await?;
        Ok(())
    }

    pub async fn users_update(
        id: i64,
        username: String,
        first_name: String,
        last_name: String,
        email: String,
        status: String,
        roles_csv: String,
        reset_required: bool,
    ) -> Result<(), ServerFnError> {
        users_update_internal(
            id,
            username,
            first_name,
            last_name,
            email,
            status,
            roles_csv,
            reset_required,
            None,
        )
        .await
    }

    pub async fn users_update_with_password(
        id: i64,
        username: String,
        first_name: String,
        last_name: String,
        email: String,
        status: String,
        roles_csv: String,
        reset_required: bool,
        new_password: String,
    ) -> Result<(), ServerFnError> {
        users_update_internal(
            id,
            username,
            first_name,
            last_name,
            email,
            status,
            roles_csv,
            reset_required,
            Some(new_password),
        )
        .await
    }

    pub async fn users_delete(id: i64) -> Result<(), ServerFnError> {
        let admin = require_admin().await?;
        let app_state = expect_context::<AppState>();

        if admin.id as i64 == id {
            return Err(ServerFnError::new("err_cannot_delete_self"));
        }

        users::Entity::delete_by_id(id)
            .exec(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?;

        Ok(())
    }

    pub async fn users_set_password_reset_required(
        id: i64,
        required: bool,
    ) -> Result<(), ServerFnError> {
        let _admin = require_admin().await?;
        let app_state = expect_context::<AppState>();

        let Some(existing) = users::Entity::find_by_id(id)
            .one(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?
        else {
            return Err(ServerFnError::new("err_user_not_found"));
        };

        let mut am: users::ActiveModel = existing.into();
        am.password_reset_required = Set(required);
        am.updated_at = Set(Utc::now());
        am.update(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?;

        Ok(())
    }

    pub async fn users_force_password_reset(id: i64) -> Result<(), ServerFnError> {
        users_set_password_reset_required(id, true).await
    }

    pub async fn users_set_enabled(id: i64, enabled: bool) -> Result<(), ServerFnError> {
        let _admin = require_admin().await?;
        let app_state = expect_context::<AppState>();

        let Some(existing) = users::Entity::find_by_id(id)
            .one(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?
        else {
            return Err(ServerFnError::new("err_user_not_found"));
        };

        let mut am: users::ActiveModel = existing.into();
        am.status = Set(if enabled {
            "active".to_string()
        } else {
            "disabled".to_string()
        });
        am.updated_at = Set(Utc::now());
        am.update(&app_state.db)
            .await
            .map_err(|_e| ServerFnError::new("err_internal"))?;

        Ok(())
    }
}

#[server(prefix = "/api/secure")]
pub async fn admin_users_list() -> Result<Vec<AdminUserRow>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        ssr::users_list().await
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::new("err_server_only"))
    }
}

#[server(prefix = "/api/secure")]
pub async fn admin_users_search(query: String) -> Result<Vec<AdminUserRow>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        ssr::users_search(query).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = query;
        Err(ServerFnError::new("err_server_only"))
    }
}

#[server(prefix = "/api/secure")]
pub async fn admin_users_create(
    csrf: String,
    username: String,
    first_name: String,
    last_name: String,
    email: String,
    password: String,
    status: String,
    roles_csv: String,
    reset_required: bool,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        crate::contexts::require_csrf(&csrf).await?;
        ssr::users_create(
            username,
            first_name,
            last_name,
            email,
            password,
            status,
            roles_csv,
            reset_required,
        )
        .await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (
            csrf,
            username,
            first_name,
            last_name,
            email,
            password,
            status,
            roles_csv,
            reset_required,
        );
        Err(ServerFnError::new("err_server_only"))
    }
}

#[server(prefix = "/api/secure")]
pub async fn admin_users_update(
    csrf: String,
    id: i64,
    username: String,
    first_name: String,
    last_name: String,
    email: String,
    status: String,
    roles_csv: String,
    reset_required: bool,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        crate::contexts::require_csrf(&csrf).await?;
        ssr::users_update(
            id,
            username,
            first_name,
            last_name,
            email,
            status,
            roles_csv,
            reset_required,
        )
        .await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (
            csrf,
            id,
            username,
            first_name,
            last_name,
            email,
            status,
            roles_csv,
            reset_required,
        );
        Err(ServerFnError::new("err_server_only"))
    }
}

#[server(prefix = "/api/secure")]
pub async fn admin_users_update_with_password(
    csrf: String,
    id: i64,
    username: String,
    first_name: String,
    last_name: String,
    email: String,
    status: String,
    roles_csv: String,
    reset_required: bool,
    new_password: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        crate::contexts::require_csrf(&csrf).await?;
        ssr::users_update_with_password(
            id,
            username,
            first_name,
            last_name,
            email,
            status,
            roles_csv,
            reset_required,
            new_password,
        )
        .await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (
            csrf,
            id,
            username,
            first_name,
            last_name,
            email,
            status,
            roles_csv,
            reset_required,
            new_password,
        );
        Err(ServerFnError::new("err_server_only"))
    }
}

#[server(prefix = "/api/secure")]
pub async fn admin_users_delete(csrf: String, id: i64) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        crate::contexts::require_csrf(&csrf).await?;
        ssr::users_delete(id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (csrf, id);
        Err(ServerFnError::new("err_server_only"))
    }
}

#[server(prefix = "/api/secure")]
pub async fn admin_users_force_password_reset(
    csrf: String,
    id: i64,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        crate::contexts::require_csrf(&csrf).await?;
        ssr::users_force_password_reset(id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (csrf, id);
        Err(ServerFnError::new("err_server_only"))
    }
}

#[server(prefix = "/api/secure")]
pub async fn admin_users_set_password_reset_required(
    csrf: String,
    id: i64,
    required: bool,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        crate::contexts::require_csrf(&csrf).await?;
        ssr::users_set_password_reset_required(id, required).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (csrf, id, required);
        Err(ServerFnError::new("err_server_only"))
    }
}

#[server(prefix = "/api/secure")]
pub async fn admin_users_set_enabled(
    csrf: String,
    id: i64,
    enabled: bool,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        crate::contexts::require_csrf(&csrf).await?;
        ssr::users_set_enabled(id, enabled).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (csrf, id, enabled);
        Err(ServerFnError::new("err_server_only"))
    }
}
