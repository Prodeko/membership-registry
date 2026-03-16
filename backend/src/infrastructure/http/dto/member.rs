use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;
use validator::Validate;

use crate::application::ports::member_repository_port::MemberWithRoles;
use crate::domain::{Email, NewPerson, Person, PersonId};

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, rename = "Member")]
pub struct MemberDTO {
    pub user_id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub full_name: Option<String>,
    pub home_municipality: String,
    pub has_accepted_policies: bool,
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
            has_accepted_policies: p.has_accepted_policies,
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
    pub home_municipality: String,
    pub has_accepted_policies: bool,
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
            has_accepted_policies: mwr.person.has_accepted_policies,
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
    #[validate(length(min = 1, max = 200))]
    pub home_municipality: String,
    pub has_accepted_policies: bool,
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
            has_accepted_policies: self.has_accepted_policies,
            email_notifications: self.email_notifications,
            language: self.language,
        })
    }
}

/// DTO for updating a member. Only contains mutable fields.
#[derive(Deserialize, Debug, TS, Validate)]
#[ts(export, rename = "UpdateMember")]
pub struct UpdateMemberDTO {
    #[validate(length(min = 1, max = 200))]
    pub first_name: String,
    #[validate(length(min = 1, max = 200))]
    pub last_name: String,
    #[validate(length(min = 1, max = 200))]
    pub home_municipality: String,
    pub has_accepted_policies: bool,
    pub email_notifications: bool,
    #[validate(length(min = 2, max = 10))]
    pub language: String,
}
