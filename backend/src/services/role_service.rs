// src/services/role_service.rs

use crate::repositories::{
    member::Member,
    role::{Role, RoleMember, RoleRepo, RoleStats, RolesWithStatsParams},
};
use futures_util::TryFutureExt;
use uuid::Uuid;

use super::{auth0_service::Auth0Service, errors::ServiceResult, member_service::MemberService};

#[derive(Clone)]
pub struct RoleService {
    pub repo: RoleRepo,
    pub member_service: MemberService,
    pub auth0_service: Auth0Service,
}

impl RoleService {
    pub fn new(repo: RoleRepo, member_service: MemberService, auth0_service: Auth0Service) -> Self {
        Self {
            repo,
            member_service,
            auth0_service,
        }
    }

    pub async fn create_role(&self, new_role: Role) -> ServiceResult<Role> {
        self.repo.create(new_role).await.map_err(|e| e.into())
    }

    pub async fn get_all_roles(&self) -> ServiceResult<Vec<Role>> {
        self.repo.fetch_all().await.map_err(|e| e.into())
    }

    pub async fn delete_role(&self, role_name: &str) -> ServiceResult<()> {
        self.repo.delete(role_name).await.map_err(|e| e.into())
    }

    pub async fn add_role_member(
        &self,
        user_id: Uuid,
        role_name: &str,
        valid_from: chrono::NaiveDate,
        valid_until: Option<chrono::NaiveDate>,
    ) -> ServiceResult<()> {
        let providers = self
            .auth0_service
            .repo
            .user_auth_provider
            .find_by_user_id(&user_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to find auth providers for user: {:?}", e);
                super::errors::ServiceError::DatabaseError
            })?;

        for provider in providers {
            self.auth0_service
                .assign_role(&provider.provider_user_id, role_name)
                .await?;
        }

        self.repo
            .create_role_member(&user_id, role_name, valid_from, valid_until)
            .await
            .map_err(|e| -> super::errors::ServiceError { e.into() })?;

        Ok(())
    }

    pub async fn get_member_roles(&self, user_id: Uuid) -> ServiceResult<Vec<RoleMember>> {
        self.repo
            .fetch_roles_by_member(&user_id)
            .await
            .map_err(|e| e.into())
    }

    pub async fn get_role_members(&self, role_name: &str) -> ServiceResult<Vec<Member>> {
        self.repo
            .fetch_members_by_role(role_name)
            .await
            .map_err(|e| e.into())
    }

    pub async fn add_many_role_members(
        &self,
        user_ids: Vec<Uuid>,
        role_names: Vec<String>,
        valid_from: chrono::NaiveDate,
        valid_until: Option<chrono::NaiveDate>,
    ) -> ServiceResult<()> {
        for role_name in role_names {
            for user_id in &user_ids {
                self.add_role_member(*user_id, role_name.as_str(), valid_from, valid_until)
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
    ) -> ServiceResult<()> {
        self.repo
            .update_valid_until(&user_id, role_name, valid_from, new_valid_until)
            .await
            .map_err(|e| e.into())
    }

    pub async fn delete_role_membership(
        &self,
        user_id: Uuid,
        role_name: &str,
        valid_from: chrono::NaiveDate,
    ) -> ServiceResult<()> {
        self.repo
            .delete_role_member(&user_id, role_name, valid_from)
            .await
            .map_err(|e| -> super::errors::ServiceError { e.into() })?;

        let providers = self
            .auth0_service
            .repo
            .user_auth_provider
            .find_by_user_id(&user_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to find auth providers for user: {:?}", e);
                super::errors::ServiceError::DatabaseError
            })?;

        for provider in providers {
            if let Err(e) = self
                .auth0_service
                .remove_role(&provider.provider_user_id, role_name)
                .await
            {
                tracing::error!("Failed to remove Auth0 role for provider {}: {:?}", provider.provider_user_id, e);
            }
        }

        Ok(())
    }

    pub async fn get_role_stats(
        &self,
        page_size: Option<u64>,
        offset: Option<u64>,
        search: Option<String>,
        order_by: Option<String>,
        order_desc: Option<bool>,
    ) -> ServiceResult<Vec<RoleStats>> {
        self.repo
            .fetch_roles_with_stats(RolesWithStatsParams {
                page_size,
                offset,
                search,
                order_by,
                order_desc,
            })
            .await
            .map_err(|e| e.into())
    }
}
