use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;
use validator::Validate;

use std::collections::HashMap;

use crate::application::ports::member_repository_port::MemberWithRoles;
use crate::application::services::role_service::MemberKeycloakSyncStatus;
use crate::domain::{Email, NewPerson, Person, PersonId};

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, rename = "Member")]
pub struct MemberDTO {
    pub user_id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub full_name: Option<String>,
    pub home_municipality: Option<String>,
    pub email_notifications: bool,
    pub language: String,
}

impl From<Person> for MemberDTO {
    fn from(p: Person) -> Self {
        Self {
            user_id: p.id.0,
            email: p.email.into_inner(),
            first_name: p.first_name,
            last_name: p.last_name,
            full_name: p.full_name,
            home_municipality: p.home_municipality,
            email_notifications: p.email_notifications,
            language: p.language,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, rename = "MemberWithRoles")]
pub struct MemberWithRolesDTO {
    pub user_id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub full_name: Option<String>,
    pub home_municipality: Option<String>,
    pub email_notifications: bool,
    pub language: String,
    pub role_names: Vec<String>,
}

impl From<MemberWithRoles> for MemberWithRolesDTO {
    fn from(mwr: MemberWithRoles) -> Self {
        Self {
            user_id: mwr.person.id.0,
            email: mwr.person.email.into_inner(),
            first_name: mwr.person.first_name,
            last_name: mwr.person.last_name,
            full_name: mwr.person.full_name,
            home_municipality: mwr.person.home_municipality,
            email_notifications: mwr.person.email_notifications,
            language: mwr.person.language,
            role_names: mwr.role_names,
        }
    }
}

#[derive(Deserialize, Debug, Clone, TS, Validate)]
#[ts(export, rename = "NewMember")]
pub struct NewMemberDTO {
    pub user_id: Uuid,
    #[validate(email, length(max = 320))]
    pub email: String,
    #[validate(length(min = 1, max = 200))]
    pub first_name: String,
    #[validate(length(min = 1, max = 200))]
    pub last_name: String,
    #[validate(length(max = 200))]
    pub home_municipality: Option<String>,
    #[serde(default = "default_true")]
    pub email_notifications: bool,
    #[serde(default = "default_language")]
    #[validate(length(min = 2, max = 10))]
    pub language: String,
}

fn default_true() -> bool {
    true
}

fn default_language() -> String {
    "fi".to_string()
}

impl NewMemberDTO {
    pub fn into_new_person(self) -> Result<NewPerson, &'static str> {
        Ok(NewPerson {
            id: PersonId(self.user_id),
            email: Email::new(self.email)?,
            first_name: self.first_name,
            last_name: self.last_name,
            home_municipality: self.home_municipality,
            email_notifications: self.email_notifications,
            language: self.language,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, rename = "MemberKeycloakSyncStatus")]
pub struct MemberKeycloakSyncStatusDTO {
    pub in_sync: bool,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub expired_in_keycloak: Vec<String>,
    pub unmanaged_in_keycloak: Vec<String>,
    pub missing_in_keycloak: Vec<String>,
}

impl From<MemberKeycloakSyncStatus> for MemberKeycloakSyncStatusDTO {
    fn from(s: MemberKeycloakSyncStatus) -> Self {
        Self {
            in_sync: s.in_sync,
            first_name: s.first_name,
            last_name: s.last_name,
            email: s.email,
            expired_in_keycloak: s.expired_in_keycloak,
            unmanaged_in_keycloak: s.unmanaged_in_keycloak,
            missing_in_keycloak: s.missing_in_keycloak,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, rename = "KeycloakSyncStatusMap")]
pub struct KeycloakSyncStatusMapDTO {
    pub statuses: HashMap<String, MemberKeycloakSyncStatusDTO>,
}

/// DTO for updating a member. Only contains mutable fields.
#[derive(Deserialize, Debug, TS, Validate)]
#[ts(export, rename = "UpdateMember")]
pub struct UpdateMemberDTO {
    #[validate(length(min = 1, max = 200))]
    pub first_name: String,
    #[validate(length(min = 1, max = 200))]
    pub last_name: String,
    #[validate(length(max = 200))]
    pub home_municipality: Option<String>,
    pub email_notifications: bool,
    #[validate(length(min = 2, max = 10))]
    pub language: String,
}
