use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use mockall::mock;
use serde_json::Value;
use uuid::Uuid;

use crate::application::ports::{
    application_repository_port::{
        ApplicationCommandPort, ApplicationQueryPort, ApplicationTargetableRole,
        ApplicationWithMember, TargetableRolePort,
    },
    audit_log_repository_port::{
        AuditLogEntryWithActor, AuditLogQueryParams, AuditLogRepositoryPort, NewAuditLogEntry,
    },
    auth_port::{AuthError, AuthPort, RefreshedTokens, VerifiedIdentity},
    auth_provider_repo_port::{
        AuthProviderMapping, AuthProviderRepoError, AuthProviderRepositoryPort,
    },
    email_port::{EmailError, EmailPort},
    marketing_list_port::{
        ContactIdentity, MarketingListError, MarketingListPort, MarketingPreferences, TagPreference,
    },
    marketing_tag_repository_port::MarketingTagRepositoryPort,
    member_repository_port::{MemberRepositoryPort, MemberWithRoles, MembersWithRolesParams},
    repository_error::RepositoryError,
    role_repository_port::{RoleMembership, RoleRepositoryPort, RoleStats, RolesWithStatsParams},
    rolesync_port::{IdpSubject, RoleSyncError, RoleSyncPort},
    template_renderer_port::TemplateRendererPort,
    template_repository_port::TemplateRepositoryPort,
    user_admin_port::{IdpUser, UserAdminError, UserAdminPort},
};
use crate::domain::{
    Application, ApplicationId, ApplicationStatus, EmailTemplate, EmailTemplateTranslation,
    MarketingTag, NewApplication, NewPerson, Person, Role, RoleName, UpdatePersonData,
};

use crate::application::services::audit_log_service::AuditLogService;

// --- ApplicationCommandPort ---

mock! {
    pub ApplicationCommandPort {}

    #[async_trait::async_trait]
    impl ApplicationCommandPort for ApplicationCommandPort {
        async fn create(&self, app: &NewApplication) -> Result<Application, RepositoryError>;
        async fn update_status(&self, application_id: Uuid, status: &ApplicationStatus) -> Result<(), RepositoryError>;
        async fn delete(&self, application_id: Uuid) -> Result<(), RepositoryError>;
        async fn update_payment_id(&self, application_id: Uuid, stripe_payment_id: String, new_status: &ApplicationStatus) -> Result<(), RepositoryError>;
    }
}

// --- ApplicationQueryPort ---

mock! {
    pub ApplicationQueryPort {}

    #[async_trait::async_trait]
    impl ApplicationQueryPort for ApplicationQueryPort {
        async fn fetch_all(&self) -> Result<Vec<Application>, RepositoryError>;
        async fn fetch_one(&self, application_id: Uuid) -> Result<Application, RepositoryError>;
        async fn fetch_with_member_one(&self, application_id: Uuid) -> Result<ApplicationWithMember, RepositoryError>;
        async fn fetch_with_user_filtered(&self, status: Option<ApplicationStatus>, search: Option<String>) -> Result<Vec<ApplicationWithMember>, RepositoryError>;
        async fn fetch_applications_for_user(&self, user_id: Uuid) -> Result<Vec<Application>, RepositoryError>;
        async fn fetch_existing(&self, user_id: Uuid, role_name: String, valid_until: NaiveDate) -> Result<Application, RepositoryError>;
    }
}

// --- TargetableRolePort ---

mock! {
    pub TargetableRolePort {}

    #[async_trait::async_trait]
    impl TargetableRolePort for TargetableRolePort {
        async fn fetch_all_targetable_roles(&self) -> Result<Vec<ApplicationTargetableRole>, RepositoryError>;
        async fn fetch_targetable_role(&self, role_name: String, valid_until: NaiveDate) -> Result<ApplicationTargetableRole, RepositoryError>;
        async fn create_targetable_role(&self, role_name: String, valid_until: NaiveDate, active: Option<bool>, payment_link: Option<String>, approved_email_template: Option<String>, rejected_email_template: Option<String>) -> Result<(), RepositoryError>;
        async fn update_targetable_role(&self, role_name: String, valid_until: NaiveDate, active: Option<bool>) -> Result<(), RepositoryError>;
        async fn delete_targetable_role(&self, role_name: String, valid_until: NaiveDate) -> Result<(), RepositoryError>;
    }
}

// --- MemberRepositoryPort ---

mock! {
    pub MemberRepositoryPort {}

    #[async_trait::async_trait]
    impl MemberRepositoryPort for MemberRepositoryPort {
        async fn create(&self, new: NewPerson) -> Result<Person, RepositoryError>;
        async fn fetch_all(&self) -> Result<Vec<Person>, RepositoryError>;
        async fn fetch_with_ids(&self, ids: Option<Vec<Uuid>>) -> Result<Vec<Person>, RepositoryError>;
        async fn fetch_one(&self, id: Uuid) -> Result<Person, RepositoryError>;
        async fn update(&self, user_id: Uuid, data: &UpdatePersonData) -> Result<Person, RepositoryError>;
        async fn delete(&self, id: Uuid) -> Result<(), RepositoryError>;
        async fn delete_many(&self, ids: Vec<Uuid>) -> Result<(), RepositoryError>;
        async fn fetch_members_with_roles(&self, params: MembersWithRolesParams) -> Result<Vec<MemberWithRoles>, RepositoryError>;
        async fn count_members_with_roles(&self, params: MembersWithRolesParams) -> Result<i64, RepositoryError>;
        async fn set_email_notifications_by_email(&self, email: &str, value: bool) -> Result<u64, RepositoryError>;
    }
}

// --- RoleRepositoryPort ---

mock! {
    pub RoleRepositoryPort {}

    #[async_trait::async_trait]
    impl RoleRepositoryPort for RoleRepositoryPort {
        async fn create(&self, role: &Role) -> Result<Role, RepositoryError>;
        async fn update(&self, role: &Role) -> Result<Role, RepositoryError>;
        async fn fetch_all(&self) -> Result<Vec<Role>, RepositoryError>;
        async fn fetch_by_name(&self, role_name: &str) -> Result<Role, RepositoryError>;
        async fn delete(&self, role_name: &str) -> Result<(), RepositoryError>;
        async fn create_role_member(&self, user_id: &Uuid, role_name: &str, valid_from: NaiveDate, valid_until: Option<NaiveDate>) -> Result<(), RepositoryError>;
        async fn create_role_members_batch(&self, user_ids: &[Uuid], role_names: &[String], valid_from: NaiveDate, valid_until: Option<NaiveDate>) -> Result<(), RepositoryError>;
        async fn update_valid_until(&self, user_id: &Uuid, role_name: &str, valid_from: NaiveDate, new_valid_until: NaiveDate) -> Result<(), RepositoryError>;
        async fn delete_role_member(&self, user_id: &Uuid, role_name: &str, valid_from: NaiveDate) -> Result<(), RepositoryError>;
        async fn fetch_roles_by_member(&self, user_id: &Uuid) -> Result<Vec<RoleMembership>, RepositoryError>;
        async fn fetch_members_by_role(&self, role_name: &str) -> Result<Vec<Person>, RepositoryError>;
        async fn fetch_roles_with_stats(&self, params: RolesWithStatsParams) -> Result<Vec<RoleStats>, RepositoryError>;
        async fn fetch_expired_unsynced(&self) -> Result<Vec<RoleMembership>, RepositoryError>;
        async fn mark_keycloak_synced(&self, user_id: &Uuid, role_name: &str, valid_from: NaiveDate) -> Result<(), RepositoryError>;
    }
}

// --- AuthPort ---

mock! {
    pub AuthPort {}

    #[async_trait::async_trait]
    impl AuthPort for AuthPort {
        async fn verify_access_token(&self, access_token: &str) -> Result<VerifiedIdentity, AuthError>;
        async fn refresh(&self, refresh_token: &str) -> Result<RefreshedTokens, AuthError>;
    }
}

// --- AuthProviderRepositoryPort ---

mock! {
    pub AuthProviderRepo {}

    #[async_trait::async_trait]
    impl AuthProviderRepositoryPort for AuthProviderRepo {
        async fn find_by_provider(&self, provider_name: &str, provider_user_id: &str) -> Result<Option<AuthProviderMapping>, AuthProviderRepoError>;
        async fn find_by_user_id(&self, user_id: &Uuid) -> Result<Vec<AuthProviderMapping>, AuthProviderRepoError>;
        async fn create(&self, user_id: &Uuid, provider_name: &str, provider_user_id: &str) -> Result<AuthProviderMapping, AuthProviderRepoError>;
        async fn delete(&self, user_id: &Uuid, provider_name: &str) -> Result<bool, AuthProviderRepoError>;
        async fn count_by_user_id(&self, user_id: &Uuid) -> Result<usize, AuthProviderRepoError>;
        async fn find_all_by_provider_name(&self, provider_name: &str) -> Result<Vec<AuthProviderMapping>, AuthProviderRepoError>;
    }
}

// --- RoleSyncPort ---

mock! {
    pub RoleSyncPort {}

    #[async_trait::async_trait]
    impl RoleSyncPort for RoleSyncPort {
        async fn create_role(&self, role: &RoleName) -> Result<(), RoleSyncError>;
        async fn delete_role(&self, role: &RoleName) -> Result<(), RoleSyncError>;
        async fn assign_role(&self, subject: &IdpSubject, role: &RoleName) -> Result<(), RoleSyncError>;
        async fn remove_role(&self, user_id: &IdpSubject, role: &RoleName) -> Result<(), RoleSyncError>;
        async fn has_role(&self, user_id: &IdpSubject, role: &RoleName) -> Result<bool, RoleSyncError>;
        async fn list_role_members(&self, role: &RoleName) -> Result<Vec<IdpSubject>, RoleSyncError>;
    }
}

// --- EmailPort ---

mock! {
    pub EmailPort {}

    #[async_trait::async_trait]
    impl EmailPort for EmailPort {
        async fn send_email(&self, to: &str, subject: &str, html_body: &str) -> Result<(), EmailError>;
    }
}

// --- MarketingListPort ---

mock! {
    pub MarketingListPort {}

    #[async_trait::async_trait]
    impl MarketingListPort for MarketingListPort {
        async fn fetch_preferences(&self, email: &str, known_tags: &[String]) -> Result<MarketingPreferences, MarketingListError>;
        async fn subscribe(&self, identity: &ContactIdentity) -> Result<(), MarketingListError>;
        async fn set_tags(&self, identity: &ContactIdentity, tag_updates: &[TagPreference]) -> Result<(), MarketingListError>;
    }
}

// --- MarketingTagRepositoryPort ---

mock! {
    pub MarketingTagRepositoryPort {}

    #[async_trait::async_trait]
    impl MarketingTagRepositoryPort for MarketingTagRepositoryPort {
        async fn fetch_all(&self) -> Result<Vec<MarketingTag>, RepositoryError>;
        async fn create(&self, tag: &MarketingTag) -> Result<MarketingTag, RepositoryError>;
        async fn update(&self, tag: &MarketingTag) -> Result<MarketingTag, RepositoryError>;
        async fn delete(&self, label: &str) -> Result<(), RepositoryError>;
    }
}

// --- TemplateRepositoryPort ---

mock! {
    pub TemplateRepositoryPort {}

    #[async_trait::async_trait]
    impl TemplateRepositoryPort for TemplateRepositoryPort {
        async fn fetch_all(&self) -> Result<Vec<EmailTemplate>, RepositoryError>;
        async fn create(&self, name: &str) -> Result<EmailTemplate, RepositoryError>;
        async fn delete(&self, name: &str) -> Result<(), RepositoryError>;
        async fn fetch_translation(&self, name: &str, locale: &str) -> Result<EmailTemplateTranslation, RepositoryError>;
        async fn fetch_translations(&self, name: &str) -> Result<Vec<EmailTemplateTranslation>, RepositoryError>;
        async fn upsert_translation(&self, template_name: &str, locale: &str, subject: &str, body_html: &str) -> Result<EmailTemplateTranslation, RepositoryError>;
        async fn delete_translation(&self, template_name: &str, locale: &str) -> Result<(), RepositoryError>;
    }
}

// --- TemplateRendererPort ---

pub struct StubRenderer;

impl TemplateRendererPort for StubRenderer {
    fn render(&self, template: &str, _variables: &[(&str, &str)]) -> String {
        template.to_string()
    }
}

mock! {
    pub TemplateRendererPort {}

    impl TemplateRendererPort for TemplateRendererPort {
        fn render<'a>(&self, template: &str, variables: &[(&'a str, &'a str)]) -> String;
    }
}

// --- AuditLogRepositoryPort ---

mock! {
    pub AuditLogRepositoryPort {}

    #[async_trait::async_trait]
    impl AuditLogRepositoryPort for AuditLogRepositoryPort {
        async fn create(&self, entry: NewAuditLogEntry) -> Result<(), RepositoryError>;
        async fn fetch_paginated(&self, params: AuditLogQueryParams) -> Result<Vec<AuditLogEntryWithActor>, RepositoryError>;
    }
}

// --- UserAdminPort ---

mock! {
    pub UserAdminPort {}

    #[async_trait::async_trait]
    impl UserAdminPort for UserAdminPort {
        async fn get_user(&self, subject: &str) -> Result<IdpUser, UserAdminError>;
        async fn update_user_locale(&self, subject: &str, locale: &str) -> Result<(), UserAdminError>;
        async fn update_user_profile(
            &self,
            subject: &str,
            first_name: &str,
            last_name: &str,
            email: Option<String>,
            require_verify_email: bool,
        ) -> Result<(), UserAdminError>;
    }
}

// --- Helpers ---

pub fn noop_audit_log() -> AuditLogService {
    let mut mock = MockAuditLogRepositoryPort::new();
    mock.expect_create().returning(|_| Ok(()));
    AuditLogService::new(Arc::new(mock))
}
