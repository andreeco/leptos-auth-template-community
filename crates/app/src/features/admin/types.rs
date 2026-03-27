use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminUserRow {
    pub id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub status: String,
    pub roles: Vec<String>,
    pub is_admin: bool,
    pub password_reset_required: bool,
    pub created_at: String,
    pub updated_at: String,
}
