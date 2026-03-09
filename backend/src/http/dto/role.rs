use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::application::ports::role_repository_port::{RoleMembership, RoleStats};
use crate::domain::Role;

#[derive(Debug, Serialize, Deserialize, Default, TS)]
#[ts(export, rename = "Role")]
pub struct RoleDTO {
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
}

impl From<Role> for RoleDTO {
    fn from(role: Role) -> Self {
        Self {
            name: role.name.0,
            color: role.color,
            description: role.description,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename = "RoleMember")]
pub struct RoleMembershipDTO {
    pub user_id: uuid::Uuid,
    pub role_name: String,
    pub valid_from: chrono::NaiveDate,
    pub valid_until: Option<chrono::NaiveDate>,
}

impl From<RoleMembership> for RoleMembershipDTO {
    fn from(rm: RoleMembership) -> Self {
        Self {
            user_id: rm.user_id,
            role_name: rm.role_name.0,
            valid_from: rm.valid_from,
            valid_until: rm.valid_until,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename = "RoleStats")]
pub struct RoleStatsDTO {
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
    #[ts(type = "number | null")]
    pub member_count: Option<i64>,
    #[ts(type = "number | null")]
    pub active_member_count: Option<i64>,
}

impl From<RoleStats> for RoleStatsDTO {
    fn from(rs: RoleStats) -> Self {
        Self {
            name: rs.name.0,
            color: rs.color,
            description: rs.description,
            member_count: rs.member_count,
            active_member_count: rs.active_member_count,
        }
    }
}
