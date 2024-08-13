use rand::seq::SliceRandom;

use crate::repositories::member::{Member, MemberRepo, MemberWithRoles, NewMember};
use serde::Deserialize;
use uuid::Uuid;

use super::ory_service::OryService;

#[derive(Clone)]
pub struct MemberService {
    pub repo: MemberRepo,
    pub ory_service: super::ory_service::OryService,
}

impl MemberService {
    pub fn new(repo: MemberRepo, ory_service: OryService) -> Self {
        Self { repo, ory_service }
    }

    pub async fn create_member(
        &self,
        member_to_add: NewMember,
        access_token: String,
    ) -> Result<Member, String> {
        let user = self
            .ory_service
            .get_user(member_to_add.user_id.to_string().as_str(), access_token)
            .await
            .map_err(|e| e.to_string());

        match user {
            Ok(_user) => self
                .repo
                .create(member_to_add)
                .await
                .map_err(|e| e.to_string()),
            Err(e) => return Err(e.to_string()),
        }
    }

    pub async fn get_all_members(&self) -> Result<Vec<Member>, String> {
        self.repo.fetch_all().await.map_err(|e| e.to_string())
    }

    pub async fn get_members_with_ids(
        &self,
        user_ids: Option<Vec<Uuid>>,
    ) -> Result<Vec<Member>, String> {
        let members = self
            .repo
            .fetch_with_ids(user_ids)
            .await
            .map_err(|e| e.to_string());

        return members;
    }

    pub async fn get_member(&self, id: Uuid) -> Result<Member, String> {
        self.repo.fetch_one(id).await.map_err(|e| e.to_string())
    }

    pub async fn get_member_with_user(
        &self,
        id: Uuid,
        access_token: String,
    ) -> Result<Member, String> {
        let user = self
            .ory_service
            .get_user(&id.to_string(), access_token)
            .await
            .map_err(|e| e.to_string())?;

        let member = self.repo.fetch_one(id).await.map_err(|e| e.to_string())?;

        let email = user
            .traits
            .clone()
            .map(|t| t.get("email").cloned())
            .flatten();

        match email {
            Some(email) => {
                return Ok(Member {
                    user_id: member.user_id,
                    first_name: member.first_name,
                    last_name: member.last_name,
                    full_name: member.full_name,
                    home_municipality: member.home_municipality,
                    has_accepted_policies: member.has_accepted_policies,
                    email: email.to_string(),
                });
            }
            None => return Err("Could not find email from user".to_string()),
        }
    }

    pub async fn update_member(
        &self,
        updated_member: Member,
        access_token: String,
    ) -> Result<Member, String> {
        let updated_user = self
            .ory_service
            .update_user(
                &updated_member.user_id.to_string(),
                &updated_member.email,
                &updated_member.first_name,
                &updated_member.last_name,
                access_token,
            )
            .await
            .map_err(|e| e.to_string());

        match updated_user {
            Ok(_) => {
                let as_member = Member {
                    user_id: updated_member.user_id,
                    email: updated_member.email,
                    first_name: updated_member.first_name,
                    last_name: updated_member.last_name,
                    full_name: updated_member.full_name,
                    home_municipality: updated_member.home_municipality,
                    has_accepted_policies: updated_member.has_accepted_policies,
                };
                self.repo
                    .update(as_member, updated_member.user_id, None)
                    .await
                    .map_err(|e| e.to_string())
                    .map(|m| Member {
                        user_id: m.user_id,
                        email: m.email,
                        first_name: m.first_name,
                        last_name: m.last_name,
                        full_name: m.full_name,
                        home_municipality: m.home_municipality,
                        has_accepted_policies: m.has_accepted_policies,
                    })
            }
            Err(e) => Err(e),
        }
    }

    pub async fn delete_member(&self, id: Uuid, access_token: String) -> Result<(), String> {
        let user_result = self
            .ory_service
            .delete_user(&id.to_string(), access_token)
            .await
            .map_err(|e| e.to_string());
        match user_result {
            Ok(_) => self.repo.delete(id).await.map_err(|e| e.to_string()),
            Err(e) => Err(e),
        }
    }

    pub async fn delete_many(&self, ids: Vec<Uuid>, access_token: String) -> Result<(), String> {
        let user_result = self
            .ory_service
            .delete_many(ids.iter().map(|id| id.to_string()).collect(), access_token)
            .await
            .map_err(|e| e.to_string());
        match user_result {
            Ok(_) => self.repo.delete_many(ids).await.map_err(|e| e.to_string()),
            Err(e) => Err(e),
        }
    }

    pub async fn get_members_with_roles(
        &self,
        page_size: Option<u64>,
        offset: Option<u64>,
        roles: Option<Vec<String>>,
        search: Option<String>,
        order_by: Option<String>,
        order_desc: Option<bool>,
        valid_from: Option<chrono::NaiveDate>,
        valid_until: Option<chrono::NaiveDate>,
    ) -> Result<Vec<MemberWithRoles>, String> {
        let members = self
            .repo
            .fetch_members_with_roles(
                page_size,
                offset,
                roles,
                search,
                order_by,
                order_desc,
                valid_from,
                valid_until,
            )
            .await
            .map_err(|e| e.to_string());

        return members;
    }
}
