use std::sync::Arc;

use chrono::{NaiveDate, Utc};
use uuid::Uuid;

use crate::application::ports::{
    application_repository_port::ApplicationTargetableRole, repository_error::RepositoryError,
};
use crate::application::services::{
    application_service::{ApplicationService, CreateApplicationParams},
    attribute_service::AttributeService,
    member_service::MemberService,
    notification_service::NotificationService,
    role_service::RoleService,
};
use crate::domain::{
    Application, ApplicationAction, ApplicationId, ApplicationStatus, NewApplication,
};

use super::mocks::*;

fn valid_until() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()
}

fn test_application(status: ApplicationStatus) -> Application {
    Application::from((
        ApplicationId(Uuid::new_v4()),
        Utc::now(),
        NewApplication {
            user_id: Uuid::new_v4(),
            role_name: "test-role".to_string(),
            valid_until: valid_until(),
            stripe_payment_id: None,
            optional_roles: None,
            application_text: None,
            status,
        },
    ))
}

fn active_targetable_role(payment_link: Option<String>) -> ApplicationTargetableRole {
    ApplicationTargetableRole {
        role_name: "test-role".to_string(),
        valid_until: valid_until(),
        active: true,
        optional_roles: None,
        payment_link,
        approved_email_template: None,
        rejected_email_template: None,
        form_attributes: vec![],
    }
}

fn build_notification_service() -> NotificationService {
    let mut template_repo = MockTemplateRepositoryPort::new();
    template_repo
        .expect_fetch_translation()
        .returning(|_, _| Err(RepositoryError::NotFound));

    let mut renderer = MockTemplateRendererPort::new();
    renderer.expect_render().returning(|t, _| t.to_string());

    NotificationService::new(None, Arc::new(template_repo), Arc::new(renderer))
}

fn build_role_service() -> RoleService {
    let role_repo = MockRoleRepositoryPort::new();
    let member_repo = MockMemberRepositoryPort::new();
    let role_sync = MockRoleSyncPort::new();
    let auth_provider_repo = MockAuthProviderRepo::new();
    let user_admin = MockUserAdminPort::new();

    let member_service = MemberService::new(
        Arc::new(member_repo),
        Arc::new(user_admin),
        Arc::new(auth_provider_repo),
        noop_audit_log(),
        None,
        noop_attribute_bootstrap(),
    );

    RoleService::new(
        Arc::new(role_repo),
        member_service,
        Arc::new(role_sync),
        Arc::new(MockAuthProviderRepo::new()),
        noop_audit_log(),
    )
}

fn build_attribute_service() -> Arc<AttributeService> {
    Arc::new(AttributeService::new(
        Arc::new(MockAttributeRepositoryPort::new()),
        Arc::new(MockAttributeSyncPort::new()),
        Arc::new(MockAuthProviderRepo::new()),
        noop_audit_log(),
    ))
}

fn build_service(
    commands: MockApplicationCommandPort,
    queries: MockApplicationQueryPort,
    targetable: MockTargetableRolePort,
) -> ApplicationService {
    build_service_with_attribute(commands, queries, targetable, build_attribute_service())
}

fn build_service_with_attribute(
    commands: MockApplicationCommandPort,
    queries: MockApplicationQueryPort,
    targetable: MockTargetableRolePort,
    attribute_service: Arc<AttributeService>,
) -> ApplicationService {
    ApplicationService::new(
        Arc::new(commands),
        Arc::new(queries),
        Arc::new(targetable),
        build_role_service(),
        attribute_service,
        noop_audit_log(),
        build_notification_service(),
    )
}

// --- create_application ---

#[tokio::test]
async fn create_application_happy_path() {
    let mut commands = MockApplicationCommandPort::new();
    let mut queries = MockApplicationQueryPort::new();
    let mut targetable = MockTargetableRolePort::new();

    queries
        .expect_fetch_existing()
        .returning(|_, _, _| Err(RepositoryError::NotFound));

    targetable
        .expect_fetch_targetable_role()
        .returning(|_, _| Ok(active_targetable_role(None)));

    commands.expect_create().returning(|new| {
        Ok(Application::from((
            ApplicationId(Uuid::new_v4()),
            Utc::now(),
            new.clone(),
        )))
    });

    let svc = build_service(commands, queries, targetable);
    let result = svc
        .create_application(
            CreateApplicationParams {
                user_id: Uuid::new_v4(),
                role_name: "test-role".to_string(),
                valid_until: valid_until(),
                stripe_payment_id: None,
                optional_roles: None,
                application_text: None,
                frontend_url: "http://localhost".to_string(),
                attributes: vec![],
            },
            None,
        )
        .await;

    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().application.status,
        ApplicationStatus::Pending
    );
}

#[tokio::test]
async fn create_application_duplicate_returns_already_exists() {
    let commands = MockApplicationCommandPort::new();
    let mut queries = MockApplicationQueryPort::new();
    let targetable = MockTargetableRolePort::new();

    queries
        .expect_fetch_existing()
        .returning(|_, _, _| Ok(test_application(ApplicationStatus::Pending)));

    let svc = build_service(commands, queries, targetable);
    let result = svc
        .create_application(
            CreateApplicationParams {
                user_id: Uuid::new_v4(),
                role_name: "test-role".to_string(),
                valid_until: valid_until(),
                stripe_payment_id: None,
                optional_roles: None,
                application_text: None,
                frontend_url: "http://localhost".to_string(),
                attributes: vec![],
            },
            None,
        )
        .await;

    assert!(matches!(
        result,
        Err(crate::application::services::errors::ServiceError::AlreadyExists)
    ));
}

#[tokio::test]
async fn create_application_targetable_not_found() {
    let commands = MockApplicationCommandPort::new();
    let mut queries = MockApplicationQueryPort::new();
    let mut targetable = MockTargetableRolePort::new();

    queries
        .expect_fetch_existing()
        .returning(|_, _, _| Err(RepositoryError::NotFound));

    targetable
        .expect_fetch_targetable_role()
        .returning(|_, _| Err(RepositoryError::NotFound));

    let svc = build_service(commands, queries, targetable);
    let result = svc
        .create_application(
            CreateApplicationParams {
                user_id: Uuid::new_v4(),
                role_name: "test-role".to_string(),
                valid_until: valid_until(),
                stripe_payment_id: None,
                optional_roles: None,
                application_text: None,
                frontend_url: "http://localhost".to_string(),
                attributes: vec![],
            },
            None,
        )
        .await;

    assert!(matches!(
        result,
        Err(crate::application::services::errors::ServiceError::NotFound)
    ));
}

#[tokio::test]
async fn create_application_inactive_role() {
    let commands = MockApplicationCommandPort::new();
    let mut queries = MockApplicationQueryPort::new();
    let mut targetable = MockTargetableRolePort::new();

    queries
        .expect_fetch_existing()
        .returning(|_, _, _| Err(RepositoryError::NotFound));

    targetable.expect_fetch_targetable_role().returning(|_, _| {
        Ok(ApplicationTargetableRole {
            active: false,
            ..active_targetable_role(None)
        })
    });

    let svc = build_service(commands, queries, targetable);
    let result = svc
        .create_application(
            CreateApplicationParams {
                user_id: Uuid::new_v4(),
                role_name: "test-role".to_string(),
                valid_until: valid_until(),
                stripe_payment_id: None,
                optional_roles: None,
                application_text: None,
                frontend_url: "http://localhost".to_string(),
                attributes: vec![],
            },
            None,
        )
        .await;

    assert!(matches!(
        result,
        Err(crate::application::services::errors::ServiceError::NotActive)
    ));
}

#[tokio::test]
async fn create_application_payment_required_sets_unpaid() {
    let mut commands = MockApplicationCommandPort::new();
    let mut queries = MockApplicationQueryPort::new();
    let mut targetable = MockTargetableRolePort::new();

    queries
        .expect_fetch_existing()
        .returning(|_, _, _| Err(RepositoryError::NotFound));

    targetable.expect_fetch_targetable_role().returning(|_, _| {
        Ok(active_targetable_role(Some(
            "https://pay.example.com".to_string(),
        )))
    });

    commands.expect_create().returning(|new| {
        Ok(Application::from((
            ApplicationId(Uuid::new_v4()),
            Utc::now(),
            new.clone(),
        )))
    });

    let svc = build_service(commands, queries, targetable);
    let result = svc
        .create_application(
            CreateApplicationParams {
                user_id: Uuid::new_v4(),
                role_name: "test-role".to_string(),
                valid_until: valid_until(),
                stripe_payment_id: None,
                optional_roles: None,
                application_text: None,
                frontend_url: "http://localhost".to_string(),
                attributes: vec![],
            },
            None,
        )
        .await;

    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().application.status,
        ApplicationStatus::Unpaid
    );
}

// --- update_application_status ---

#[tokio::test]
async fn update_status_approve_calls_role_assignment() {
    let mut commands = MockApplicationCommandPort::new();
    let mut queries = MockApplicationQueryPort::new();
    let mut targetable = MockTargetableRolePort::new();

    let app = test_application(ApplicationStatus::Pending);
    let app_id = app.application_id.0;
    let user_id = app.user_id;

    queries
        .expect_fetch_one()
        .returning(move |_| Ok(test_application(ApplicationStatus::Pending)));

    commands
        .expect_update_status()
        .withf(|_, status| *status == ApplicationStatus::Approved)
        .returning(|_, _| Ok(()));

    // For send_status_notification
    targetable
        .expect_fetch_targetable_role()
        .returning(|_, _| Ok(active_targetable_role(None)));
    queries
        .expect_fetch_with_member_one()
        .returning(|_| Err(RepositoryError::NotFound));

    // Build role service that expects add_role_member flow
    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo
        .expect_create_role_member()
        .returning(|_, _, _, _| Ok(()));

    let mut auth_provider_repo = MockAuthProviderRepo::new();
    auth_provider_repo
        .expect_find_by_user_id()
        .returning(|_| Ok(vec![]));

    let member_repo = MockMemberRepositoryPort::new();
    let user_admin = MockUserAdminPort::new();
    let role_sync = MockRoleSyncPort::new();

    let member_service = MemberService::new(
        Arc::new(member_repo),
        Arc::new(user_admin),
        Arc::new(MockAuthProviderRepo::new()),
        noop_audit_log(),
        None,
        noop_attribute_bootstrap(),
    );

    let role_service = RoleService::new(
        Arc::new(role_repo),
        member_service,
        Arc::new(role_sync),
        Arc::new(auth_provider_repo),
        noop_audit_log(),
    );

    let svc = ApplicationService::new(
        Arc::new(commands),
        Arc::new(queries),
        Arc::new(targetable),
        role_service,
        build_attribute_service(),
        noop_audit_log(),
        build_notification_service(),
    );

    let result = svc
        .update_application_status(app_id, ApplicationAction::Approve, None)
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn update_status_approve_succeeds_when_idp_sync_fails() {
    use crate::application::ports::auth_provider_repo_port::AuthProviderMapping;
    use crate::application::ports::rolesync_port::RoleSyncError;

    let mut commands = MockApplicationCommandPort::new();
    let mut queries = MockApplicationQueryPort::new();
    let mut targetable = MockTargetableRolePort::new();

    let app = test_application(ApplicationStatus::Pending);
    let app_id = app.application_id.0;
    let user_id = app.user_id;

    queries
        .expect_fetch_one()
        .returning(move |_| Ok(test_application(ApplicationStatus::Pending)));

    commands
        .expect_update_status()
        .withf(|_, status| *status == ApplicationStatus::Approved)
        .returning(|_, _| Ok(()));

    targetable
        .expect_fetch_targetable_role()
        .returning(|_, _| Ok(active_targetable_role(None)));
    queries
        .expect_fetch_with_member_one()
        .returning(|_| Err(RepositoryError::NotFound));

    // DB role membership must still be written even though IdP will fail.
    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo
        .expect_create_role_member()
        .times(1)
        .returning(|_, _, _, _| Ok(()));

    // Auth provider exists, so we will attempt IdP sync.
    let mut auth_provider_repo = MockAuthProviderRepo::new();
    auth_provider_repo
        .expect_find_by_user_id()
        .returning(move |_| {
            Ok(vec![AuthProviderMapping {
                user_id,
                provider_name: "keycloak".to_string(),
                provider_user_id: "kc-subject-123".to_string(),
                linked_at: Utc::now(),
            }])
        });

    // IdP role sync fails — approval must still succeed.
    let mut role_sync = MockRoleSyncPort::new();
    role_sync
        .expect_assign_role()
        .times(1)
        .returning(|_, _| Err(RoleSyncError::IdpError));

    let member_repo = MockMemberRepositoryPort::new();
    let user_admin = MockUserAdminPort::new();

    let member_service = MemberService::new(
        Arc::new(member_repo),
        Arc::new(user_admin),
        Arc::new(MockAuthProviderRepo::new()),
        noop_audit_log(),
        None,
        noop_attribute_bootstrap(),
    );

    let role_service = RoleService::new(
        Arc::new(role_repo),
        member_service,
        Arc::new(role_sync),
        Arc::new(auth_provider_repo),
        noop_audit_log(),
    );

    let svc = ApplicationService::new(
        Arc::new(commands),
        Arc::new(queries),
        Arc::new(targetable),
        role_service,
        build_attribute_service(),
        noop_audit_log(),
        build_notification_service(),
    );

    let result = svc
        .update_application_status(app_id, ApplicationAction::Approve, None)
        .await;

    assert!(
        result.is_ok(),
        "Approval should succeed even when IdP role sync fails: {result:?}"
    );
}

#[tokio::test]
async fn update_status_reject_does_not_assign_role() {
    let mut commands = MockApplicationCommandPort::new();
    let mut queries = MockApplicationQueryPort::new();
    let mut targetable = MockTargetableRolePort::new();

    let app = test_application(ApplicationStatus::Pending);
    let app_id = app.application_id.0;

    queries
        .expect_fetch_one()
        .returning(move |_| Ok(test_application(ApplicationStatus::Pending)));

    commands
        .expect_update_status()
        .withf(|_, status| *status == ApplicationStatus::Rejected)
        .returning(|_, _| Ok(()));

    targetable
        .expect_fetch_targetable_role()
        .returning(|_, _| Ok(active_targetable_role(None)));
    queries
        .expect_fetch_with_member_one()
        .returning(|_| Err(RepositoryError::NotFound));

    // role_repo.create_role_member should NOT be called
    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo.expect_create_role_member().never();

    let member_repo = MockMemberRepositoryPort::new();
    let user_admin = MockUserAdminPort::new();
    let role_sync = MockRoleSyncPort::new();
    let auth_provider_repo = MockAuthProviderRepo::new();

    let member_service = MemberService::new(
        Arc::new(member_repo),
        Arc::new(user_admin),
        Arc::new(MockAuthProviderRepo::new()),
        noop_audit_log(),
        None,
        noop_attribute_bootstrap(),
    );

    let role_service = RoleService::new(
        Arc::new(role_repo),
        member_service,
        Arc::new(role_sync),
        Arc::new(auth_provider_repo),
        noop_audit_log(),
    );

    let svc = ApplicationService::new(
        Arc::new(commands),
        Arc::new(queries),
        Arc::new(targetable),
        role_service,
        build_attribute_service(),
        noop_audit_log(),
        build_notification_service(),
    );

    let result = svc
        .update_application_status(app_id, ApplicationAction::Reject, None)
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn update_status_payment_received() {
    let mut commands = MockApplicationCommandPort::new();
    let mut queries = MockApplicationQueryPort::new();
    let targetable = MockTargetableRolePort::new();

    queries
        .expect_fetch_one()
        .returning(move |_| Ok(test_application(ApplicationStatus::Unpaid)));

    commands
        .expect_update_status()
        .withf(|_, status| *status == ApplicationStatus::Pending)
        .returning(|_, _| Ok(()));

    let svc = build_service(commands, queries, targetable);
    let result = svc
        .update_application_status(Uuid::new_v4(), ApplicationAction::PaymentReceived, None)
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn update_status_already_terminal() {
    let commands = MockApplicationCommandPort::new();
    let mut queries = MockApplicationQueryPort::new();
    let targetable = MockTargetableRolePort::new();

    queries
        .expect_fetch_one()
        .returning(move |_| Ok(test_application(ApplicationStatus::Approved)));

    let svc = build_service(commands, queries, targetable);
    let result = svc
        .update_application_status(Uuid::new_v4(), ApplicationAction::Approve, None)
        .await;

    assert!(matches!(
        result,
        Err(crate::application::services::errors::ServiceError::ApplicationAlreadyProcessed)
    ));
}

#[tokio::test]
async fn update_status_invalid_action() {
    let commands = MockApplicationCommandPort::new();
    let mut queries = MockApplicationQueryPort::new();
    let targetable = MockTargetableRolePort::new();

    queries
        .expect_fetch_one()
        .returning(move |_| Ok(test_application(ApplicationStatus::Pending)));

    let svc = build_service(commands, queries, targetable);
    let result = svc
        .update_application_status(Uuid::new_v4(), ApplicationAction::PaymentReceived, None)
        .await;

    assert!(matches!(
        result,
        Err(crate::application::services::errors::ServiceError::InvalidStatus)
    ));
}

// --- delete_application ---

#[tokio::test]
async fn delete_application_happy_path() {
    let mut commands = MockApplicationCommandPort::new();
    let queries = MockApplicationQueryPort::new();
    let targetable = MockTargetableRolePort::new();

    commands.expect_delete().returning(|_| Ok(()));

    let svc = build_service(commands, queries, targetable);
    let result = svc.delete_application(Uuid::new_v4(), None).await;

    assert!(result.is_ok());
}

// --- form attributes on application creation ---

#[tokio::test]
async fn create_application_rejects_attribute_not_on_form() {
    use crate::domain::{AttributeName, AttributeValue};

    let mut commands = MockApplicationCommandPort::new();
    let mut queries = MockApplicationQueryPort::new();
    let mut targetable = MockTargetableRolePort::new();

    queries
        .expect_fetch_existing()
        .returning(|_, _, _| Err(RepositoryError::NotFound));

    // Form lists no attributes — submitting any should be rejected before
    // we reach the application_commands.create call.
    targetable
        .expect_fetch_targetable_role()
        .returning(|_, _| Ok(active_targetable_role(None)));

    commands.expect_create().times(0);

    let svc = build_service(commands, queries, targetable);
    let result = svc
        .create_application(
            CreateApplicationParams {
                user_id: Uuid::new_v4(),
                role_name: "test-role".to_string(),
                valid_until: valid_until(),
                stripe_payment_id: None,
                optional_roles: None,
                application_text: None,
                frontend_url: "http://localhost".to_string(),
                attributes: vec![(
                    AttributeName::new("major-subject").unwrap(),
                    AttributeValue::new("iem").unwrap(),
                )],
            },
            None,
        )
        .await;

    assert!(matches!(
        result,
        Err(crate::application::services::errors::ServiceError::Constraint(_))
    ));
}

#[tokio::test]
async fn create_application_writes_submitted_attribute_to_member() {
    use crate::application::ports::application_repository_port::ApplicationTargetableRole;
    use crate::domain::{AttributeDefinition, AttributeName, AttributeValue, EditableBy};
    use std::sync::Mutex;

    let user_id = Uuid::new_v4();

    let mut commands = MockApplicationCommandPort::new();
    let mut queries = MockApplicationQueryPort::new();
    let mut targetable = MockTargetableRolePort::new();

    queries
        .expect_fetch_existing()
        .returning(|_, _, _| Err(RepositoryError::NotFound));

    targetable.expect_fetch_targetable_role().returning(|_, _| {
        Ok(ApplicationTargetableRole {
            role_name: "test-role".to_string(),
            valid_until: valid_until(),
            active: true,
            optional_roles: None,
            payment_link: None,
            approved_email_template: None,
            rejected_email_template: None,
            form_attributes: vec![AttributeName::new("major-subject").unwrap()],
        })
    });

    commands.expect_create().returning(|new| {
        Ok(Application::from((
            ApplicationId(Uuid::new_v4()),
            Utc::now(),
            new.clone(),
        )))
    });

    // The attribute service path must produce one upsert against MemberAttribute
    // for the submitted value, bypassing editable_by because the form is its
    // own authorization context.
    let mut attr_repo = MockAttributeRepositoryPort::new();
    attr_repo.expect_fetch_definition().returning(|_| {
        Ok(Some(
            AttributeDefinition::new(
                AttributeName::new("major-subject").unwrap(),
                None,
                Some(vec![
                    AttributeValue::new("iem").unwrap(),
                    AttributeValue::new("other").unwrap(),
                ]),
                None,
                false,
                EditableBy::Admin, // admin-only on profile, but form bypasses
            )
            .unwrap(),
        ))
    });
    let upserts: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));
    let upserts_for_repo = Arc::clone(&upserts);
    attr_repo
        .expect_upsert_member_value()
        .returning(move |_uid, name, value| {
            upserts_for_repo
                .lock()
                .unwrap()
                .push((name.as_str().to_string(), value.as_str().to_string()));
            Ok(())
        });

    let attr_service = Arc::new(AttributeService::new(
        Arc::new(attr_repo),
        Arc::new(MockAttributeSyncPort::new()),
        Arc::new(MockAuthProviderRepo::new()),
        noop_audit_log(),
    ));

    let svc = build_service_with_attribute(commands, queries, targetable, attr_service);
    let result = svc
        .create_application(
            CreateApplicationParams {
                user_id,
                role_name: "test-role".to_string(),
                valid_until: valid_until(),
                stripe_payment_id: None,
                optional_roles: None,
                application_text: None,
                frontend_url: "http://localhost".to_string(),
                attributes: vec![(
                    AttributeName::new("major-subject").unwrap(),
                    AttributeValue::new("iem").unwrap(),
                )],
            },
            None,
        )
        .await;

    assert!(result.is_ok());
    let recorded = upserts.lock().unwrap().clone();
    assert_eq!(
        recorded,
        vec![("major-subject".to_string(), "iem".to_string())]
    );
}
