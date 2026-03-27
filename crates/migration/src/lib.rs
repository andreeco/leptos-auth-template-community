pub use sea_orm_migration::prelude::*;

mod m20260327_070930_create_users;
mod m20260327_070930_create_roles;
mod m20260327_070930_create_permissions;
mod m20260327_070930_create_user_roles;
mod m20260327_070930_create_role_permissions;
mod m20260327_070930_create_webauthn_credentials;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260327_070930_create_users::Migration),
            Box::new(m20260327_070930_create_roles::Migration),
            Box::new(m20260327_070930_create_permissions::Migration),
            Box::new(m20260327_070930_create_user_roles::Migration),
            Box::new(m20260327_070930_create_role_permissions::Migration),
            Box::new(m20260327_070930_create_webauthn_credentials::Migration),
        ]
    }
}
