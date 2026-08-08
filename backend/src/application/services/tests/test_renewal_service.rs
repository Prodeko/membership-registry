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
async fn rejects_role_not_configured_for_renewal() {
    let user_id = Uuid::new_v4();
    for broken in [
        Role {
            renewable: false,
            ..renewable_role(30, 0)
        },
        Role {
            renewal_payment_link: None,
            ..renewable_role(30, 0)
        },
        Role {
            renewal_period_months: Some(0),
            ..renewable_role(30, 0)
        },
    ] {
        let mut role_repo = MockRoleRepositoryPort::new();
        role_repo
            .expect_fetch_by_name()
            .returning(move |_| Ok(broken.clone()));
        role_repo.expect_fetch_roles_by_member().never();

        let mut renewal_repo = MockRoleRenewalRepositoryPort::new();
        renewal_repo.expect_find_pending().never();
        renewal_repo.expect_create().never();

        let svc = make_service(renewal_repo, role_repo);
        let result = svc.start_member_renewal(user_id, ROLE).await;
        assert!(matches!(result, Err(ServiceError::Constraint(_))));
    }
}

#[tokio::test]
async fn picks_latest_membership_when_several_exist() {
    let user_id = Uuid::new_v4();
    let mut old = membership(user_id, -350);
    old.valid_from = Utc::now().date_naive() - Duration::days(1000);
    let latest = membership(user_id, 15);
    let latest_valid_from = latest.valid_from;

    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo
        .expect_fetch_by_name()
        .returning(|_| Ok(renewable_role(30, 0)));
    let rows = vec![old, latest];
    role_repo
        .expect_fetch_roles_by_member()
        .returning(move |_| Ok(rows.clone()));

    let mut renewal_repo = MockRoleRenewalRepositoryPort::new();
    renewal_repo
        .expect_find_pending()
        .withf(move |_, _, valid_from| *valid_from == latest_valid_from)
        .returning(|_, _, _| Ok(None));
    renewal_repo
        .expect_create()
        .withf(move |r| r.old_valid_from == latest_valid_from)
        .returning(|r| Ok(r.clone()));

    let svc = make_service(renewal_repo, role_repo);
    assert!(svc.start_member_renewal(user_id, ROLE).await.is_ok());
}

#[tokio::test]
async fn recovers_pending_renewal_when_unique_index_race_fires() {
    let user_id = Uuid::new_v4();
    let m = membership(user_id, 15);
    let winner = pending_renewal(user_id, &m);
    let winner_id = winner.renewal_id;

    let mut role_repo = MockRoleRepositoryPort::new();
    role_repo
        .expect_fetch_by_name()
        .returning(|_| Ok(renewable_role(30, 0)));
    let m_clone = m.clone();
    role_repo
        .expect_fetch_roles_by_member()
        .returning(move |_| Ok(vec![m_clone.clone()]));

    let mut renewal_repo = MockRoleRenewalRepositoryPort::new();
    let mut find_pending_results = vec![Ok(Some(winner)), Ok(None)];
    renewal_repo
        .expect_find_pending()
        .times(2)
        .returning(move |_, _, _| find_pending_results.pop().unwrap());
    renewal_repo.expect_create().returning(|_| {
        Err(crate::application::ports::repository_error::RepositoryError::Constraint(
            "duplicate key value violates unique constraint \
             \"idx_role_renewal_pending_unique\""
                .to_string(),
        ))
    });

    let svc = make_service(renewal_repo, role_repo);
    let url = svc.start_member_renewal(user_id, ROLE).await.unwrap();
    assert_eq!(
        url,
        format!("https://buy.stripe.com/test?client_reference_id={winner_id}")
    );
}

#[tokio::test]
async fn propagates_non_race_constraint_errors() {
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
    // Only the pre-create lookup: an FK violation must NOT trigger the
    // race-recovery re-fetch
    renewal_repo
        .expect_find_pending()
        .times(1)
        .returning(|_, _, _| Ok(None));
    renewal_repo.expect_create().returning(|_| {
        Err(crate::application::ports::repository_error::RepositoryError::Constraint(
            "violates foreign key constraint \"rolerenewal_user_id_fkey\"".to_string(),
        ))
    });

    let svc = make_service(renewal_repo, role_repo);
    let result = svc.start_member_renewal(user_id, ROLE).await;
    assert!(
        matches!(result, Err(ServiceError::Constraint(ref msg)) if msg.contains("foreign key"))
    );
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
