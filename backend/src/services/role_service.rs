// src/services/role_service.rs

use crate::repositories::{
    member::Member,
    role::{Role, RoleMember, RoleRepo},
};
use uuid::Uuid;

use super::member_service::MemberService;

pub struct RoleService {
    pub repo: RoleRepo,
    pub member_service: MemberService,
}

impl RoleService {
    pub fn new(repo: RoleRepo, member_service: MemberService) -> Self {
        Self {
            repo,
            member_service,
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
    ) -> Result<(), String> {
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
    ) -> Result<(), String> {
        self.repo
            .delete_role_member(user_id, role_name, valid_from)
            .await
            .map_err(|e| e.to_string())
    }
}
