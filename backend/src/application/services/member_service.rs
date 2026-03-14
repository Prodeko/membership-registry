use std::sync::Arc;

use crate::application::ports::{
    auth_provider_repo_port::AuthProviderRepositoryPort,
    member_repository_port::{MemberRepositoryPort, MemberWithRoles, MembersWithRolesParams},
    user_admin_port::UserAdminPort,
};
use crate::domain::{Email, NewPerson, Person, UpdatePersonData};
use uuid::Uuid;

use super::{
    audit_log_service::AuditLogService,
    errors::{ServiceError, ServiceResult},
};

#[derive(Clone)]
pub struct MemberService {
    pub member_repo: Arc<dyn MemberRepositoryPort>,
    pub user_admin: Arc<dyn UserAdminPort>,
    pub auth_provider_repo: Arc<dyn AuthProviderRepositoryPort>,
    pub audit_log: AuditLogService,
}

impl MemberService {
    pub fn new(
        member_repo: Arc<dyn MemberRepositoryPort>,
        user_admin: Arc<dyn UserAdminPort>,
        auth_provider_repo: Arc<dyn AuthProviderRepositoryPort>,
        audit_log: AuditLogService,
    ) -> Self {
        Self {
            member_repo,
            user_admin,
            auth_provider_repo,
            audit_log,
        }
    }

    pub async fn create_member(
        &self,
        new_person: NewPerson,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<Person> {
        let person = self
            .member_repo
            .create(new_person)
            .await
            .map_err(ServiceError::from)?;

        self.audit_log
            .log(
                actor_user_id,
                "member.create",
                "member",
                &person.id.0.to_string(),
                Some(serde_json::json!({
                    "email": person.email.as_str(),
                    "first_name": person.first_name,
                    "last_name": person.last_name,
                })),
            )
            .await;

        Ok(person)
    }

    pub async fn get_all_members(&self) -> ServiceResult<Vec<Person>> {
        self.member_repo
            .fetch_all()
            .await
            .map_err(ServiceError::from)
    }

    pub async fn get_members_with_ids(
        &self,
        user_ids: Option<Vec<Uuid>>,
    ) -> ServiceResult<Vec<Person>> {
        self.member_repo
            .fetch_with_ids(user_ids)
            .await
            .map_err(ServiceError::from)
    }

    pub async fn get_member(&self, id: Uuid) -> ServiceResult<Person> {
        self.member_repo
            .fetch_one(id)
            .await
            .map_err(ServiceError::from)
    }

    pub async fn get_member_with_user(&self, id: Uuid) -> ServiceResult<Person> {
        let auth_providers = self
            .auth_provider_repo
            .find_by_user_id(&id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to find auth providers: {e:?}");
                ServiceError::DatabaseError
            })?;

        if auth_providers.is_empty() {
            return Err(ServiceError::NotFound);
        }

        let primary_provider = &auth_providers[0];

        let user = self
            .user_admin
            .get_user(&primary_provider.provider_user_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get IdP user: {e:?}");
                ServiceError::IdpError
            })?;

        let member = self
            .member_repo
            .fetch_one(id)
            .await
            .map_err(ServiceError::from)?;

        let email = user
            .email
            .and_then(|e| Email::new(e).ok())
            .unwrap_or(member.email.clone());

        Ok(Person {
            id: member.id,
            first_name: member.first_name,
            last_name: member.last_name,
            full_name: member.full_name,
            home_municipality: member.home_municipality,
            has_accepted_policies: member.has_accepted_policies,
            email_notifications: member.email_notifications,
            email,
        })
    }

    pub async fn update_member(
        &self,
        user_id: Uuid,
        data: UpdatePersonData,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<Person> {
        let updated = self
            .member_repo
            .update(user_id, &data)
            .await
            .map_err(ServiceError::from)?;

        self.audit_log
            .log(
                actor_user_id,
                "member.update",
                "member",
                &updated.id.0.to_string(),
                Some(serde_json::json!({
                    "email": updated.email.as_str(),
                    "first_name": updated.first_name,
                    "last_name": updated.last_name,
                })),
            )
            .await;

        Ok(updated)
    }

    pub async fn delete_member(&self, id: Uuid, actor_user_id: Option<Uuid>) -> ServiceResult<()> {
        self.member_repo
            .delete(id)
            .await
            .map_err(ServiceError::from)?;

        self.audit_log
            .log(
                actor_user_id,
                "member.delete",
                "member",
                &id.to_string(),
                None,
            )
            .await;

        Ok(())
    }

    pub async fn delete_many(
        &self,
        ids: Vec<Uuid>,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        self.member_repo
            .delete_many(ids.clone())
            .await
            .map_err(ServiceError::from)?;

        self.audit_log
            .log(
                actor_user_id,
                "member.delete_many",
                "member",
                "batch",
                Some(serde_json::json!({ "deleted_member_ids": ids })),
            )
            .await;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
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
        self.member_repo
            .fetch_members_with_roles(MembersWithRolesParams {
                valid_from,
                valid_until,
                roles,
                page_size,
                offset,
                search,
                order_by,
                order_desc,
            })
            .await
            .map_err(ServiceError::from)
    }

    pub async fn log_export(&self, actor_user_id: Option<Uuid>) {
        self.audit_log
            .log(actor_user_id, "member.export_csv", "member", "export", None)
            .await;
    }
}
