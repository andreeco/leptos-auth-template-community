#[cfg(not(feature = "ssr"))]
fn main() {
    eprintln!(
        "This binary requires the `ssr` feature.\nRun:\n  cargo run --features ssr --bin seed_admin"
    );
    std::process::exit(1);
}

#[cfg(feature = "ssr")]
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Database, EntityTrait, IntoActiveModel, QueryFilter, Set,
};

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ---- Safety gate --------------------------------------------------------
    if !env_bool("SEED_ALLOW_ADMIN") {
        eprintln!(
            "Refusing to seed admin without explicit opt-in.\n\
             Set SEED_ALLOW_ADMIN=1 and run again."
        );
        std::process::exit(2);
    }

    // ---- Config -------------------------------------------------------------
    let db_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://app.sqlite?mode=rwc".into());

    let dev_insecure = env_bool("SEED_DEV_INSECURE");
    let force_reset = env_bool("SEED_ADMIN_FORCE_RESET");

    let seed_staff = env_bool("SEED_STAFF");
    let staff_force_reset = env_bool("SEED_STAFF_FORCE_RESET");
    let staff_username = std::env::var("SEED_STAFF_USERNAME")
        .unwrap_or_else(|_| "staff".to_string())
        .trim()
        .to_string();

    if seed_staff && staff_username.eq_ignore_ascii_case("admin") {
        return Err("SEED_STAFF_USERNAME must not be 'admin'".into());
    }

    let (plain_password, source) = resolve_password("SEED_ADMIN_PASSWORD", dev_insecure)?;
    let staff_seed_input = if seed_staff {
        let (pw, src) = resolve_password("SEED_STAFF_PASSWORD", dev_insecure)?;
        Some((pw, src))
    } else {
        None
    };

    let db = Database::connect(&db_url).await?;

    // ---- Upsert admin user --------------------------------------------------
    let hash = password_auth::generate_hash(&plain_password);
    let (admin_user, user_action, password_applied) = upsert_admin_user(&db, &hash, force_reset).await?;

    // Ensure user has stable webauthn_user_handle for passkey enrollment
    let admin_user = ensure_webauthn_handle(&db, admin_user).await?;

    // ---- Optional staff user ------------------------------------------------
    let seeded_staff = if let Some((staff_plain_password, staff_source)) = staff_seed_input {
        let staff_hash = password_auth::generate_hash(&staff_plain_password);
        let (staff_user, staff_action, staff_password_applied) =
            upsert_staff_user(&db, &staff_username, &staff_hash, staff_force_reset).await?;
        let staff_user = ensure_webauthn_handle(&db, staff_user).await?;

        Some((
            staff_user,
            staff_action,
            staff_password_applied,
            staff_plain_password,
            staff_source,
        ))
    } else {
        None
    };

    // ---- Upsert roles -------------------------------------------------------
    let user_role_id = upsert_role(&db, "user").await?;
    let admin_role_id = upsert_role(&db, "admin").await?;
    let staff_role_id = upsert_role(&db, "staff").await?;

    upsert_user_role(&db, admin_user.id, user_role_id).await?;
    upsert_user_role(&db, admin_user.id, admin_role_id).await?;

    if let Some((staff_user, _, _, _, _)) = &seeded_staff {
        upsert_user_role(&db, staff_user.id, user_role_id).await?;
        upsert_user_role(&db, staff_user.id, staff_role_id).await?;
    }

    // ---- Optional baseline permission mapping -------------------------------
    let admin_read_perm_id = upsert_permission(&db, "admin.read").await?;
    upsert_role_permission(&db, admin_role_id, admin_read_perm_id).await?;

    if seeded_staff.is_some() {
        let staff_read_perm_id = upsert_permission(&db, "staff.read").await?;
        upsert_role_permission(&db, staff_role_id, staff_read_perm_id).await?;
    }

    // ---- Report -------------------------------------------------------------
    let mut summary = if password_applied {
        format!(
            "Seed admin result:\n\
             - action={}\n\
             - username=admin\n\
             - password_source={}\n\
             - password_applied=true\n\
             - roles=[user,admin]\n\
             - permission=[admin.read]\n\
             - webauthn_user_handle={}\n\
             \n\
             Effective plaintext credential (store securely):\n\
             admin={}\n",
            user_action,
            password_source_label(source),
            admin_user
                .webauthn_user_handle
                .clone()
                .unwrap_or_else(|| "<missing>".to_string()),
            plain_password
        )
    } else {
        format!(
            "Seed admin result:\n\
             - action={}\n\
             - username=admin\n\
             - password_source={}\n\
             - password_applied=false (existing password kept; set SEED_ADMIN_FORCE_RESET=1 to rotate)\n\
             - roles=[user,admin]\n\
             - permission=[admin.read]\n\
             - webauthn_user_handle={}\n",
            user_action,
            password_source_label(source),
            admin_user
                .webauthn_user_handle
                .clone()
                .unwrap_or_else(|| "<missing>".to_string())
        )
    };

    if let Some((staff_user, staff_action, staff_password_applied, staff_plain_password, staff_source)) = seeded_staff {
        if staff_password_applied {
            summary.push_str(&format!(
                "\nSeed staff result:\n\
                 - action={}\n\
                 - username={}\n\
                 - password_source={}\n\
                 - password_applied=true\n\
                 - roles=[user,staff]\n\
                 - permission=[staff.read]\n\
                 - webauthn_user_handle={}\n\
                 \n\
                 Effective plaintext credential (store securely):\n\
                 {}={}\n",
                staff_action,
                staff_username,
                password_source_label(staff_source),
                staff_user
                    .webauthn_user_handle
                    .clone()
                    .unwrap_or_else(|| "<missing>".to_string()),
                staff_username,
                staff_plain_password
            ));
        } else {
            summary.push_str(&format!(
                "\nSeed staff result:\n\
                 - action={}\n\
                 - username={}\n\
                 - password_source={}\n\
                 - password_applied=false (existing password kept; set SEED_STAFF_FORCE_RESET=1 to rotate)\n\
                 - roles=[user,staff]\n\
                 - permission=[staff.read]\n\
                 - webauthn_user_handle={}\n",
                staff_action,
                staff_username,
                password_source_label(staff_source),
                staff_user
                    .webauthn_user_handle
                    .clone()
                    .unwrap_or_else(|| "<missing>".to_string())
            ));
        }
    }

    println!("{summary}");

    if let Ok(path) = std::env::var("SEED_ADMIN_OUT_FILE") {
        let path = path.trim();
        if !path.is_empty() {
            if let Some(parent) = std::path::Path::new(path).parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            std::fs::write(path, summary.as_bytes())?;
            println!("Wrote seed summary to {path}");
        }
    }

    Ok(())
}

#[cfg(feature = "ssr")]
#[derive(Debug, Clone, Copy)]
enum PasswordSource {
    ExplicitEnv,
    GeneratedRandom,
    DevInsecureDefault,
}

#[cfg(feature = "ssr")]
fn env_bool(name: &str) -> bool {
    let raw = std::env::var(name).unwrap_or_default();
    matches!(
        raw.trim().to_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

#[cfg(feature = "ssr")]
fn resolve_password(
    env_name: &str,
    dev_insecure: bool,
) -> Result<(String, PasswordSource), Box<dyn std::error::Error>> {
    if let Ok(v) = std::env::var(env_name) {
        let pw = v.trim().to_string();
        if pw.is_empty() {
            return Err(format!("{env_name} was provided but empty").into());
        }
        if !dev_insecure {
            validate_strong_password(env_name, &pw)?;
        }
        return Ok((pw, PasswordSource::ExplicitEnv));
    }

    if dev_insecure {
        return Ok(("password".to_string(), PasswordSource::DevInsecureDefault));
    }

    Ok((generate_random_passphrase("admin"), PasswordSource::GeneratedRandom))
}

#[cfg(feature = "ssr")]
fn validate_strong_password(name: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
    let lower = value.to_lowercase();
    if matches!(lower.as_str(), "password" | "admin" | "user" | "changeme") {
        return Err(format!("{name} uses a forbidden weak/default value").into());
    }
    if value.len() < 12 {
        return Err(format!("{name} must be at least 12 characters in secure mode").into());
    }
    Ok(())
}

#[cfg(feature = "ssr")]
fn generate_random_passphrase(label: &str) -> String {
    let a = uuid::Uuid::new_v4().simple().to_string();
    let b = uuid::Uuid::new_v4().simple().to_string();
    format!("example-app-{}-{}-{}", label, &a[..12], &b[..12])
}

#[cfg(feature = "ssr")]
fn password_source_label(src: PasswordSource) -> &'static str {
    match src {
        PasswordSource::ExplicitEnv => "explicit_env",
        PasswordSource::GeneratedRandom => "generated_random",
        PasswordSource::DevInsecureDefault => "dev_insecure_default",
    }
}

#[cfg(feature = "ssr")]
async fn upsert_admin_user(
    db: &sea_orm::DatabaseConnection,
    password_hash: &str,
    force_reset: bool,
) -> Result<(leptos_auth_template_community::entities::users::Model, &'static str, bool), sea_orm::DbErr> {
    upsert_user_by_username(db, "admin", password_hash, force_reset).await
}

#[cfg(feature = "ssr")]
async fn upsert_staff_user(
    db: &sea_orm::DatabaseConnection,
    username: &str,
    password_hash: &str,
    force_reset: bool,
) -> Result<(leptos_auth_template_community::entities::users::Model, &'static str, bool), sea_orm::DbErr> {
    upsert_user_by_username(db, username, password_hash, force_reset).await
}

#[cfg(feature = "ssr")]
async fn upsert_user_by_username(
    db: &sea_orm::DatabaseConnection,
    username: &str,
    password_hash: &str,
    force_reset: bool,
) -> Result<(leptos_auth_template_community::entities::users::Model, &'static str, bool), sea_orm::DbErr> {
    use leptos_auth_template_community::entities::users;

    let now = chrono::Utc::now();

    if let Some(existing) = users::Entity::find()
        .filter(users::Column::Username.eq(username))
        .one(db)
        .await?
    {
        if !force_reset {
            return Ok((existing, "existing_kept", false));
        }

        let mut am: users::ActiveModel = existing.into_active_model();
        am.password_hash = Set(password_hash.to_string());
        am.updated_at = Set(now);
        let updated = am.update(db).await?;
        return Ok((updated, "existing_password_reset", true));
    }

    let created = users::ActiveModel {
        username: Set(username.to_string()),
        first_name: Set(username.to_string()),
        last_name: Set(String::new()),
        email: Set(format!("{username}@example.invalid")),
        password_hash: Set(password_hash.to_string()),
        status: Set("active".to_string()),
        password_reset_required: Set(false),
        webauthn_user_handle: Set(Some(uuid::Uuid::new_v4().to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok((created, "created", true))
}

#[cfg(feature = "ssr")]
async fn ensure_webauthn_handle(
    db: &sea_orm::DatabaseConnection,
    user: leptos_auth_template_community::entities::users::Model,
) -> Result<leptos_auth_template_community::entities::users::Model, sea_orm::DbErr> {
    if user.webauthn_user_handle.is_some() {
        return Ok(user);
    }

    let mut am: leptos_auth_template_community::entities::users::ActiveModel = user.into_active_model();
    am.webauthn_user_handle = Set(Some(uuid::Uuid::new_v4().to_string()));
    am.updated_at = Set(chrono::Utc::now());
    am.update(db).await
}

#[cfg(feature = "ssr")]
async fn upsert_role(
    db: &sea_orm::DatabaseConnection,
    name: &str,
) -> Result<i64, sea_orm::DbErr> {
    use leptos_auth_template_community::entities::roles;

    if let Some(r) = roles::Entity::find()
        .filter(roles::Column::Name.eq(name))
        .one(db)
        .await?
    {
        return Ok(r.id);
    }

    let r = roles::ActiveModel {
        name: Set(name.to_string()),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(r.id)
}

#[cfg(feature = "ssr")]
async fn upsert_permission(
    db: &sea_orm::DatabaseConnection,
    name: &str,
) -> Result<i64, sea_orm::DbErr> {
    use leptos_auth_template_community::entities::permissions;

    if let Some(p) = permissions::Entity::find()
        .filter(permissions::Column::Name.eq(name))
        .one(db)
        .await?
    {
        return Ok(p.id);
    }

    let p = permissions::ActiveModel {
        name: Set(name.to_string()),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(p.id)
}

#[cfg(feature = "ssr")]
async fn upsert_user_role(
    db: &sea_orm::DatabaseConnection,
    user_id: i64,
    role_id: i64,
) -> Result<(), sea_orm::DbErr> {
    use leptos_auth_template_community::entities::user_roles;

    let exists = user_roles::Entity::find_by_id((user_id, role_id))
        .one(db)
        .await?
        .is_some();

    if !exists {
        user_roles::ActiveModel {
            user_id: Set(user_id),
            role_id: Set(role_id),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }

    Ok(())
}

#[cfg(feature = "ssr")]
async fn upsert_role_permission(
    db: &sea_orm::DatabaseConnection,
    role_id: i64,
    permission_id: i64,
) -> Result<(), sea_orm::DbErr> {
    use leptos_auth_template_community::entities::role_permissions;

    let exists = role_permissions::Entity::find_by_id((role_id, permission_id))
        .one(db)
        .await?
        .is_some();

    if !exists {
        role_permissions::ActiveModel {
            role_id: Set(role_id),
            permission_id: Set(permission_id),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }

    Ok(())
}
