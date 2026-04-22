use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::domain::{RoleGroup, RoleGroupMembership};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename = "RoleGroup")]
pub struct RoleGroupDTO {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub role_names: Vec<String>,
}

impl From<RoleGroup> for RoleGroupDTO {
    fn from(g: RoleGroup) -> Self {
        Self {
            id: g.id.0,
            name: g.name,
            description: g.description,
            role_names: g.role_names.into_iter().map(|r| r.0).collect(),
        }
    }
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, rename = "CreateRoleGroupRequest")]
pub struct CreateRoleGroupRequestDTO {
    pub name: String,
    pub description: Option<String>,
    pub role_names: Vec<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, rename = "UpdateRoleGroup")]
pub struct UpdateRoleGroupDTO {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, rename = "SetRoleGroupRoles")]
pub struct SetRoleGroupRolesDTO {
    pub role_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename = "RoleGroupMembership")]
pub struct RoleGroupMembershipDTO {
    pub group_id: Uuid,
    pub group_name: String,
    pub user_id: Uuid,
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
}

impl From<RoleGroupMembership> for RoleGroupMembershipDTO {
    fn from(m: RoleGroupMembership) -> Self {
        Self {
            group_id: m.group_id,
            group_name: m.group_name,
            user_id: m.user_id,
            valid_from: m.valid_from,
            valid_until: m.valid_until,
        }
    }
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, rename = "AssignRoleGroupRequest")]
pub struct AssignRoleGroupRequestDTO {
    pub user_id: Uuid,
    pub valid_from: Option<NaiveDate>,
    pub valid_until: Option<NaiveDate>,
}
