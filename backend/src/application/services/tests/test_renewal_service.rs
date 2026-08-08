use std::sync::Arc;

use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::application::ports::role_repository_port::RoleMembership;
use crate::application::services::audit_log_service::AuditLogService;
use crate::application::services::errors::ServiceError;
use crate::application::services::notification_service::NotificationService;
use crate::application::services::renewal_service::RenewalService;
use crate::domain::{RenewalStatus, Role, RoleName, RoleRenewal};

use super::mocks::*;

const ROLE: &str = "member";

fn renewable_role(window: i32, grace: i32) -> Role {
    Role {
        name: RoleName(ROLE.to_string()),
        color: None,
        description: None,
        renewable: true,
        renewal_payment_link: Some("https://buy.stripe.com/test".to_string()),
        renewal_period_months: Some(12),
        renewal_email_template: None,
        renewal_notification_days: vec![30, 7, 1],
        renewal_window_days: window,
        grace_period_days: grace,
    }
}

fn membership(user_id: Uuid, days_until_expiry: i64) -> RoleMembership {
    let today = Utc::now().date_naive();
    RoleMembership {
        user_id,
        role_name: RoleName(ROLE.to_string()),
        valid_from: today - Duration::days(300),
        valid_until: Some(today + Duration::days(days_until_expiry)),
        renewable: true,
        renewal_payment_link: Some("https://buy.stripe.com/test".to_string()),
        pending_renewal_id: None,
        renewal_due: true,
        renewal_deadline: Some(today + Duration::days(days_until_expiry)),
    }
}

fn pending_renewal(user_id: Uuid, membership: &RoleMembership) -> RoleRenewal {
    let old_valid_until = membership.valid_until.unwrap();
    RoleRenewal {
        renewal_id: Uuid::new_v4(),
        user_id,
        role_name: RoleName(ROLE.to_string()),
        old_valid_from: membership.valid_from,
        old_valid_until,
        new_valid_from: old_valid_until + Duration::days(1),
        new_valid_until: old_valid_until + Duration::days(366),
        status: RenewalStatus::Pending,
        stripe_payment_id: None,
        notified_days: Vec::new(),
    }
}

fn make_service(
    renewal_repo: MockRoleRenewalRepositoryPort,
    role_repo: MockRoleRepositoryPort,
) -> RenewalService {
    let mut audit_repo = MockAuditLogRepositoryPort::new();
    audit_repo.expect_create().returning(|_| Ok(()));

    let notification = NotificationService::new(
        None,
        Arc::new(MockTemplateRepositoryPort::new()),
        Arc::new(MockTemplateRendererPort::new()),
    );

    RenewalService::new(
        Arc::new(renewal_repo),
        Arc::new(role_repo),
        Arc::new(MockRoleSyncPort::new()),
        Arc::new(MockAuthProviderRepo::new()),
        notification,
        AuditLogService::new(Arc::new(audit_repo)),
    )
}

#[tokio::test]
async fn rejects_when_window_not_open() {
    let user_id = Uuid::new_v4();
    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo
        .expect_fetch_by_name()
        .returning(|_| Ok(renewable_role(30, 0)));
    role_repo
        .expect_fetch_roles_by_member()
        .returning(move |_| Ok(vec![membership(user_id, 60)]));

    let svc = make_service(MockRoleRenewalRepositoryPort::new(), role_repo);

    let result = svc.start_member_renewal(user_id, ROLE).await;
    assert!(matches!(result, Err(ServiceError::Constraint(_))));
}

#[tokio::test]
async fn returns_existing_pending_renewal_url() {
    let user_id = Uuid::new_v4();
    let m = membership(user_id, 15);
    let existing = pending_renewal(user_id, &m);
    let existing_id = existing.renewal_id;

    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo
        .expect_fetch_by_name()
        .returning(|_| Ok(renewable_role(30, 0)));
    let m_clone = m.clone();
    role_repo
        .expect_fetch_roles_by_member()
        .returning(move |_| Ok(vec![m_clone.clone()]));

    let mut renewal_repo = MockRoleRenewalRepositoryPort::new();
    renewal_repo
        .expect_find_pending()
        .returning(move |_, _, _| Ok(Some(existing.clone())));
    renewal_repo.expect_create().never();

    let svc = make_service(renewal_repo, role_repo);

    let url = svc.start_member_renewal(user_id, ROLE).await.unwrap();
    assert_eq!(
        url,
        format!("https://buy.stripe.com/test?client_reference_id={existing_id}")
    );
}

#[tokio::test]
async fn creates_renewal_when_none_pending() {
    let user_id = Uuid::new_v4();
    let m = membership(user_id, 15);

    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo
        .expect_fetch_by_name()
        .returning(|_| Ok(renewable_role(30, 0)));
    let m_clone = m.clone();
    role_repo
        .expect_fetch_roles_by_member()
        .returning(move |_| Ok(vec![m_clone.clone()]));

    let mut renewal_repo = MockRoleRenewalRepositoryPort::new();
    renewal_repo
        .expect_find_pending()
        .returning(|_, _, _| Ok(None));
    renewal_repo
        .expect_create()
        .withf(move |r| {
            r.user_id == user_id
                && r.role_name.0 == ROLE
                && r.status == RenewalStatus::Pending
                && r.new_valid_from == r.old_valid_until + Duration::days(1)
        })
        .returning(|r| Ok(r.clone()));

    let svc = make_service(renewal_repo, role_repo);

    let url = svc.start_member_renewal(user_id, ROLE).await.unwrap();
    assert!(url.starts_with("https://buy.stripe.com/test?client_reference_id="));
}

#[tokio::test]
async fn allows_renewal_during_grace_period() {
    let user_id = Uuid::new_v4();
    let m = membership(user_id, -5);

    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo
        .expect_fetch_by_name()
        .returning(|_| Ok(renewable_role(30, 14)));
    let m_clone = m.clone();
    role_repo
        .expect_fetch_roles_by_member()
        .returning(move |_| Ok(vec![m_clone.clone()]));

    let mut renewal_repo = MockRoleRenewalRepositoryPort::new();
    renewal_repo
        .expect_find_pending()
        .returning(|_, _, _| Ok(None));
    renewal_repo.expect_create().returning(|r| Ok(r.clone()));

    let svc = make_service(renewal_repo, role_repo);

    assert!(svc.start_member_renewal(user_id, ROLE).await.is_ok());
}
