use std::sync::Arc;

use chrono::{Datelike, Duration, Utc};
use uuid::Uuid;

use crate::application::ports::role_repository_port::RoleMembership;
use crate::application::services::member_service::MemberService;
use crate::application::services::role_service::RoleService;
use crate::domain::{add_months, RenewalPrompt, Role, RoleName};

use super::mocks::*;

fn make_role_service(role_repo: MockRoleRepositoryPort) -> RoleService {
    let member_service = MemberService::new(
        Arc::new(MockMemberRepositoryPort::new()),
        Arc::new(MockUserAdminPort::new()),
        Arc::new(MockAuthProviderRepo::new()),
        noop_audit_log(),
        None,
        noop_attribute_bootstrap(),
    );

    RoleService::new(
        Arc::new(role_repo),
        member_service,
        Arc::new(MockRoleSyncPort::new()),
        Arc::new(MockAuthProviderRepo::new()),
        noop_audit_log(),
    )
}

fn renewable_role() -> Role {
    Role {
        name: RoleName("membership".to_string()),
        color: None,
        description: None,
        renewable: true,
        renewal_payment_link: Some("https://pay".to_string()),
        renewal_period_months: Some(12),
        renewal_email_template: None,
        renewal_notification_days: vec![30, 7, 1],
        renewal_window_days: 45,
        grace_period_days: 14,
    }
}

fn due_membership(user_id: Uuid) -> RoleMembership {
    let today = Utc::now().date_naive();
    RoleMembership {
        user_id,
        role_name: RoleName("membership".to_string()),
        valid_from: today - Duration::days(300),
        valid_until: Some(today + Duration::days(10)),
        renewable: true,
        renewal_due: true,
        renewal_deadline: Some(today + Duration::days(24)),
    }
}

#[tokio::test]
async fn renders_prompts_with_substituted_year_for_due_roles() {
    let user_id = Uuid::new_v4();
    let m = due_membership(user_id);
    let valid_until = m.valid_until.unwrap();
    let expected_year = add_months(valid_until + Duration::days(1), 12).year();

    let mut role_repo = MockRoleRepositoryPort::new();
    let m_clone = m.clone();
    role_repo
        .expect_fetch_roles_by_member()
        .returning(move |_| Ok(vec![m_clone.clone()]));
    role_repo
        .expect_fetch_by_name()
        .returning(|_| Ok(renewable_role()));
    role_repo.expect_fetch_renewal_prompts().returning(|_| {
        Ok(vec![RenewalPrompt {
            locale: "fi".to_string(),
            title: "Jäsenmaksu vuodelle {year}".to_string(),
            body: "Et ole maksanut jäsenmaksua vuodelle {year}. Uusi {deadline} mennessä."
                .to_string(),
            button_label: "Maksa jäsenmaksu".to_string(),
        }])
    });

    let svc = make_role_service(role_repo);

    let views = svc.get_member_roles(user_id).await.unwrap();
    assert_eq!(views.len(), 1);
    let prompts = &views[0].renewal_prompts;
    assert_eq!(prompts.len(), 1);
    assert_eq!(
        prompts[0].title,
        format!("Jäsenmaksu vuodelle {expected_year}")
    );
    assert!(prompts[0].body.contains(&expected_year.to_string()));
    let deadline = valid_until + Duration::days(14);
    assert!(prompts[0].body.contains(&format!(
        "{}.{}.{}",
        deadline.day(),
        deadline.month(),
        deadline.year()
    )));
}

#[tokio::test]
async fn non_due_roles_get_no_prompts_and_no_role_fetch() {
    let user_id = Uuid::new_v4();
    let mut m = due_membership(user_id);
    m.renewal_due = false;

    let mut role_repo = MockRoleRepositoryPort::new();
    let m_clone = m.clone();
    role_repo
        .expect_fetch_roles_by_member()
        .returning(move |_| Ok(vec![m_clone.clone()]));
    role_repo.expect_fetch_by_name().never();
    role_repo.expect_fetch_renewal_prompts().never();

    let svc = make_role_service(role_repo);

    let views = svc.get_member_roles(user_id).await.unwrap();
    assert!(views[0].renewal_prompts.is_empty());
}
