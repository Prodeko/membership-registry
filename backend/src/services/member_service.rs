use std::f64::consts::E;

use rand::seq::SliceRandom;

use crate::repositories::member::{Member, MemberRepo, MemberWithRoles, MembersWithRolesParams, NewMember};
use serde::Deserialize;
use uuid::Uuid;

use super::{auth0_service::Auth0Service, errors::{ServiceError, ServiceResult}};

#[derive(Clone)]
pub struct MemberService {
    pub repo: MemberRepo,
    pub auth0_service: Auth0Service,
}

impl MemberService {
    pub fn new(repo: MemberRepo, auth0_service: Auth0Service) -> Self {
        Self { repo, auth0_service }
    }

    pub async fn create_member(
        &self,
        member_to_add: NewMember,
        access_token: String,
    ) -> ServiceResult<Member> {
        self.repo
            .create(member_to_add)
            .await
            .map_err(|e| e.into())
    }

    pub async fn get_all_members(&self) -> ServiceResult<Vec<Member>> {
        self.repo.fetch_all().await.map_err(|e| e.into())
    }

    pub async fn get_members_with_ids(
        &self,
        user_ids: Option<Vec<Uuid>>,
    ) -> ServiceResult<Vec<Member>> {
         self
            .repo
            .fetch_with_ids(user_ids)
            .await
            .map_err(|e| e.into())
    }

    pub async fn get_member(&self, id: Uuid) -> ServiceResult<Member> {
        self.repo.fetch_one(id).await.map_err(|e| e.into())
    }

    pub async fn get_member_with_user(
        &self,
        id: Uuid,
        access_token: String,
    ) -> ServiceResult<Member> {
        let auth_providers = self
            .auth0_service
            .repo
            .user_auth_provider
            .find_by_user_id(&id)
            .await?;

        if auth_providers.is_empty() {
            return Err(ServiceError::NotFound);
        }

        let primary_provider = &auth_providers[0];
        let auth0_user_id = format!("{}|{}", primary_provider.provider_name, primary_provider.provider_user_id.split('|').last().unwrap_or(&primary_provider.provider_user_id));

        let user = self
            .auth0_service
            .get_user(&primary_provider.provider_user_id)
            .await?;

        let member = self.repo.fetch_one(id).await?;

        let email = user.email.unwrap_or(member.email);

        Ok(Member {
            user_id: member.user_id,
            first_name: member.first_name,
            last_name: member.last_name,
            full_name: member.full_name,
            home_municipality: member.home_municipality,
            has_accepted_policies: member.has_accepted_policies,
            email,
        })
    }

    pub async fn update_member(
        &self,
        updated_member: Member,
        access_token: String,
    ) -> ServiceResult<Member> {
        let auth_providers = self
            .auth0_service
            .repo
            .user_auth_provider
            .find_by_user_id(&updated_member.user_id)
            .await?;

        if let Some(primary_provider) = auth_providers.first() {
            self.auth0_service
                .update_user(
                    &primary_provider.provider_user_id,
                    Some(&updated_member.email),
                    Some(&updated_member.first_name),
                    Some(&updated_member.last_name),
                )
                .await?;
        }

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
            .map(|m| Member {
                user_id: m.user_id,
                email: m.email,
                first_name: m.first_name,
                last_name: m.last_name,
                full_name: m.full_name,
                home_municipality: m.home_municipality,
                has_accepted_policies: m.has_accepted_policies,
            })
            .map_err(|e| e.into())
    }

    pub async fn delete_member(&self, id: Uuid, access_token: String) -> ServiceResult<()> {
        let auth_providers = self
            .auth0_service
            .repo
            .user_auth_provider
            .find_by_user_id(&id)
            .await?;

        for provider in auth_providers {
            self.auth0_service
                .delete_user(&provider.provider_user_id)
                .await?;
        }

        self.repo.delete(id).await.map_err(|e| e.into())
    }

    pub async fn delete_many(&self, ids: Vec<Uuid>, access_token: String) -> ServiceResult<()> {
        for id in &ids {
            let auth_providers = self
                .auth0_service
                .repo
                .user_auth_provider
                .find_by_user_id(id)
                .await?;

            for provider in auth_providers {
                self.auth0_service
                    .delete_user(&provider.provider_user_id)
                    .await?;
            }
        }

        self.repo.delete_many(ids).await.map_err(|e| e.into())
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
    ) -> ServiceResult<Vec<MemberWithRoles>> {
        self
            .repo
            .fetch_members_with_roles(
                MembersWithRolesParams {
                    valid_from,
                    valid_until,
                    roles,
                    page_size,
                    offset,
                    search,
                    order_by,
                    order_desc,
                },
            )
            .await.map_err(|e| e.into())
    }
}
