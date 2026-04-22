use chrono::NaiveDate;
use uuid::Uuid;

use crate::domain::RoleName;

#[derive(Debug, Clone)]
pub struct RoleGroupId(pub Uuid);

#[derive(Debug, Clone)]
pub struct RoleGroup {
    pub id: RoleGroupId,
    pub name: String,
    pub description: Option<String>,
    pub keycloak_group_id: Option<String>,
    pub role_names: Vec<RoleName>,
}

#[derive(Debug, Clone)]
pub struct RoleGroupMembership {
    pub group_id: Uuid,
    pub group_name: String,
    pub user_id: Uuid,
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
}
