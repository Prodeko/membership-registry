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
    pub renewable: bool,
    pub renewal_payment_link: Option<String>,
    pub renewal_period_months: Option<i32>,
    pub renewal_email_template: Option<String>,
    pub renewal_notification_days: Vec<i32>,
    pub renewal_window_days: i32,
    pub grace_period_days: i32,
}

impl From<Role> for RoleDTO {
    fn from(role: Role) -> Self {
        Self {
            name: role.name.0,
            color: role.color,
            description: role.description,
            renewable: role.renewable,
            renewal_payment_link: role.renewal_payment_link,
            renewal_period_months: role.renewal_period_months,
            renewal_email_template: role.renewal_email_template,
            renewal_notification_days: role.renewal_notification_days,
            renewal_window_days: role.renewal_window_days,
            grace_period_days: role.grace_period_days,
        }
    }
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, rename = "UpdateRole")]
pub struct UpdateRoleDTO {
    pub color: Option<String>,
    pub description: Option<String>,
    pub renewable: bool,
    pub renewal_payment_link: Option<String>,
    pub renewal_period_months: Option<i32>,
    pub renewal_email_template: Option<String>,
    pub renewal_notification_days: Vec<i32>,
    pub renewal_window_days: i32,
    pub grace_period_days: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename = "RoleMember")]
pub struct RoleMembershipDTO {
    pub user_id: uuid::Uuid,
    pub role_name: String,
    pub valid_from: chrono::NaiveDate,
    pub valid_until: Option<chrono::NaiveDate>,
    pub renewable: bool,
    pub renewal_payment_link: Option<String>,
    pub pending_renewal_id: Option<uuid::Uuid>,
    pub renewal_due: bool,
    pub renewal_deadline: Option<chrono::NaiveDate>,
}

impl From<RoleMembership> for RoleMembershipDTO {
    fn from(rm: RoleMembership) -> Self {
        Self {
            user_id: rm.user_id,
            role_name: rm.role_name.0,
            valid_from: rm.valid_from,
            valid_until: rm.valid_until,
            renewable: rm.renewable,
            renewal_payment_link: rm.renewal_payment_link,
            pending_renewal_id: rm.pending_renewal_id,
            renewal_due: rm.renewal_due,
            renewal_deadline: rm.renewal_deadline,
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[ts(export, rename = "StartRenewalResponse")]
pub struct StartRenewalResponseDTO {
    pub payment_url: String,
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
