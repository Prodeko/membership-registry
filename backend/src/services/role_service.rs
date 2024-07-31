// src/services/role_service.rs

use crate::repositories::{
    member::Member,
    role::{self, Role, RoleMember, RoleRepo},
};
use futures_util::TryFutureExt;
use uuid::Uuid;

use super::{
    member_service::MemberService,
    ory_service::{self, OryService},
};

pub struct RoleService {
    pub repo: RoleRepo,
    pub member_service: MemberService,
    pub ory_service: OryService,
}

impl RoleService {
    pub fn new(repo: RoleRepo, member_service: MemberService, ory_service: OryService) -> Self {
        Self {
            repo,
            member_service,
            ory_service,
        }
    }

    pub async fn create_role(&self, role_name: &str) -> Result<Role, String> {
        self.repo.create(role_name).await.map_err(|e| e.to_string())
    }

    pub async fn get_all_roles(&self) -> Result<Vec<Role>, String> {
        self.repo.fetch_all().await.map_err(|e| e.to_string())
    }

    pub async fn delete_role(&self, role_name: &str) -> Result<(), String> {
        self.repo.delete(role_name).await.map_err(|e| e.to_string())
    }

    pub async fn add_role_member(
        &self,
        user_id: Uuid,
        role_name: &str,
        valid_from: chrono::NaiveDate,
        valid_until: Option<chrono::NaiveDate>,
        access_token: String,
    ) -> Result<(), String> {
        self.ory_service
            .add_user_to_group(user_id.to_string(), role_name, access_token)
            .await?;

        self.repo
            .create_role_member(user_id, role_name, valid_from, valid_until)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn get_member_roles(&self, user_id: Uuid) -> Result<Vec<RoleMember>, String> {
        self.repo
            .fetch_roles_by_member(user_id)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn get_role_members(&self, role_name: &str) -> Result<Vec<Member>, String> {
        self.repo
            .fetch_members_by_role(role_name)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn add_many_role_members(
        &self,
        user_ids: Vec<Uuid>,
        role_names: Vec<String>,
        valid_from: chrono::NaiveDate,
        valid_until: Option<chrono::NaiveDate>,
        access_token: String,
    ) -> Result<(), String> {
        for role_name in role_names {
            for user_id in &user_ids {
                self.add_role_member(*user_id, role_name.as_str(), valid_from, valid_until, access_token.clone())
                    .map_err(|e| e.to_string())
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn update_role_member(
        &self,
        user_id: Uuid,
        role_name: &str,
        valid_from: chrono::NaiveDate,
        new_valid_until: chrono::NaiveDate,
    ) -> Result<(), String> {
        self.repo
            .update_valid_until(user_id, role_name, valid_from, new_valid_until)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn delete_role_membership(
        &self,
        user_id: Uuid,
        role_name: &str,
        valid_from: chrono::NaiveDate,
        access_token: String
    ) -> Result<(), String> {
        self.ory_service
            .remove_user_from_group(user_id.to_string(), role_name, access_token)
            .await?;
        
        self.repo
            .delete_role_member(user_id, role_name, valid_from)
            .await
            .map_err(|e| e.to_string())
    }
}
