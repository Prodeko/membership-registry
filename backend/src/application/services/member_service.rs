use std::sync::Arc;

use crate::application::ports::{
    auth_provider_repo_port::AuthProviderRepositoryPort,
    marketing_list_port::UpsertOutcome,
    member_repository_port::{MemberRepositoryPort, MemberWithRoles, MembersWithRolesParams},
    user_admin_port::UserAdminPort,
};
use crate::domain::{Email, NewPerson, Person, UpdatePersonData};
use uuid::Uuid;

/// Result of `MemberService::update_member`. Carries the updated person plus
/// the outcome of the synchronous Mailchimp push (if any). `marketing_outcome`
/// is `Some(PendingConfirmation)` when Mailchimp required a fallback to
/// `status: pending` — the HTTP layer surfaces this to the UI so the user
/// knows to confirm the opt-in email.
pub struct UpdateMemberResult {
    pub person: Person,
    pub marketing_outcome: Option<UpsertOutcome>,
}

use super::{
    audit_log_service::AuditLogService,
    errors::{ServiceError, ServiceResult},
    marketing_sync_service::MarketingSyncService,
};

#[derive(Clone)]
pub struct MemberService {
    pub member_repo: Arc<dyn MemberRepositoryPort>,
    pub user_admin: Arc<dyn UserAdminPort>,
    pub auth_provider_repo: Arc<dyn AuthProviderRepositoryPort>,
    pub audit_log: AuditLogService,
    pub marketing_sync: MarketingSyncService,
}

impl MemberService {
    pub fn new(
        member_repo: Arc<dyn MemberRepositoryPort>,
        user_admin: Arc<dyn UserAdminPort>,
        auth_provider_repo: Arc<dyn AuthProviderRepositoryPort>,
        audit_log: AuditLogService,
        marketing_sync: MarketingSyncService,
    ) -> Self {
        Self {
            member_repo,
            user_admin,
            auth_provider_repo,
            audit_log,
            marketing_sync,
        }
    }

    pub async fn create_member(
        &self,
        new_person: NewPerson,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<Person> {
        let language = new_person.language.clone();
        let user_id = new_person.id.0;
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

        // Fire-and-forget sync of locale to Keycloak
        let auth_providers = self
            .auth_provider_repo
            .find_by_user_id(&user_id)
            .await
            .map_err(|e| tracing::warn!("Failed to fetch auth providers for locale sync: {e:?}"))
            .ok();

        if let Some(providers) = auth_providers {
            if let Some(primary) = providers.first() {
                let subject = primary.provider_user_id.clone();
                let user_admin = Arc::clone(&self.user_admin);
                tokio::spawn(async move {
                    if let Err(e) = user_admin.update_user_locale(&subject, &language).await {
                        tracing::error!("Failed to sync locale to Keycloak: {e:?}");
                    }
                });
            }
        }

        self.marketing_sync.push_contact_async(person.clone());

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
                ServiceError::DatabaseError(format!("{e:?}"))
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
            language: member.language,
            email,
        })
    }

    pub async fn update_member(
        &self,
        user_id: Uuid,
        data: UpdatePersonData,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<UpdateMemberResult> {
        // Fetch the previous state so we can detect a resubscribe transition
        // (false -> true on email_notifications). That's the only path where
        // Mailchimp might fall back to `pending` and we need to tell the UI.
        // On any other kind of edit we spawn the marketing push to keep the
        // request fast — request latency shouldn't be coupled to Mailchimp.
        let previous = self
            .member_repo
            .fetch_one(user_id)
            .await
            .map_err(ServiceError::from)?;
        let resubscribing = !previous.email_notifications && data.email_notifications;

        let language = data.language.clone();
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

        // Fire-and-forget sync of locale to Keycloak
        let auth_providers = self
            .auth_provider_repo
            .find_by_user_id(&user_id)
            .await
            .map_err(|e| tracing::warn!("Failed to fetch auth providers for locale sync: {e:?}"))
            .ok();

        if let Some(providers) = auth_providers {
            if let Some(primary) = providers.first() {
                let subject = primary.provider_user_id.clone();
                let user_admin = Arc::clone(&self.user_admin);
                tokio::spawn(async move {
                    if let Err(e) = user_admin.update_user_locale(&subject, &language).await {
                        tracing::error!("Failed to sync locale to Keycloak: {e:?}");
                    }
                });
            }
        }

        let marketing_outcome = if resubscribing {
            self.marketing_sync
                .push_contact_awaited(updated.clone())
                .await
        } else {
            self.marketing_sync.push_contact_async(updated.clone());
            None
        };

        Ok(UpdateMemberResult {
            person: updated,
            marketing_outcome,
        })
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
