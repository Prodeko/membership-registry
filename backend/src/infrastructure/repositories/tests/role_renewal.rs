#[cfg(test)]
mod test_role_renewal {
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use crate::application::ports::repository_error::RepositoryError;
    use crate::application::ports::role_renewal_repository_port::RoleRenewalRepositoryPort;
    use crate::application::ports::role_repository_port::RoleRepositoryPort;
    use crate::domain::{RenewalStatus, Role, RoleName, RoleRenewal};
    use crate::infrastructure::repositories::tests::{cleanup_test_db, setup_test_db};
    use crate::infrastructure::repositories::PostgresRepo;

    const ROLE_NAME: &str = "renewal-test-role";

    fn user_id() -> Uuid {
        Uuid::parse_str("9707582e-c149-45a7-bae1-4b0f4de4b06f").unwrap()
    }

    /// Creates a renewable role, a membership expiring in 10 days, and returns
    /// an unsaved pending RoleRenewal for that membership.
    async fn seed(repo: &PostgresRepo) -> RoleRenewal {
        sqlx::query("UPDATE Member SET email_notifications = TRUE WHERE user_id = $1")
            .bind(user_id())
            .execute(&repo.member.pool)
            .await
            .unwrap();

        repo.role
            .create(&Role {
                name: RoleName(ROLE_NAME.to_string()),
                color: None,
                description: None,
                renewable: true,
                renewal_payment_link: Some("https://buy.stripe.com/test".to_string()),
                renewal_period_months: Some(12),
                renewal_email_template: None,
                renewal_notification_days: vec![14],
                renewal_window_days: 30,
                grace_period_days: 0,
            })
            .await
            .unwrap();

        let today = Utc::now().date_naive();
        let valid_from = today - Duration::days(300);
        let valid_until = today + Duration::days(10);

        repo.role
            .create_role_member(&user_id(), ROLE_NAME, valid_from, Some(valid_until))
            .await
            .unwrap();

        RoleRenewal {
            renewal_id: Uuid::new_v4(),
            user_id: user_id(),
            role_name: RoleName(ROLE_NAME.to_string()),
            old_valid_from: valid_from,
            old_valid_until: valid_until,
            new_valid_from: valid_until + Duration::days(1),
            new_valid_until: valid_until + Duration::days(366),
            status: RenewalStatus::Pending,
            stripe_payment_id: None,
            notified_days: Vec::new(),
        }
    }

    #[tokio::test]
    async fn mark_notified_appends_arbitrary_offsets_once() {
        let (repo, db_url) = setup_test_db().await;
        let renewal = seed(&repo).await;
        let created = repo.role_renewal.create(&renewal).await.unwrap();

        repo.role_renewal
            .mark_notified(created.renewal_id, 14)
            .await
            .unwrap();
        repo.role_renewal
            .mark_notified(created.renewal_id, 14)
            .await
            .unwrap();

        let fetched = repo
            .role_renewal
            .find_by_id(created.renewal_id)
            .await
            .unwrap();
        assert_eq!(fetched.notified_days, vec![14]);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn notification_query_reports_days_left_and_notified_offsets() {
        let (repo, db_url) = setup_test_db().await;
        let renewal = seed(&repo).await;
        let created = repo.role_renewal.create(&renewal).await.unwrap();

        // Expiry is 10 days out: inside a 14-day horizon, outside a 7-day one
        let due = repo
            .role_renewal
            .find_pending_needing_notification(ROLE_NAME, 14)
            .await
            .unwrap();
        let row = due
            .iter()
            .find(|p| p.renewal_id == created.renewal_id)
            .unwrap();
        assert_eq!(row.days_left, 10);
        assert!(row.notified_days.is_empty());

        let not_due = repo
            .role_renewal
            .find_pending_needing_notification(ROLE_NAME, 7)
            .await
            .unwrap();
        assert!(not_due.iter().all(|p| p.renewal_id != created.renewal_id));

        // Milestone filtering is the caller's job: a notified renewal is still
        // returned, with the sent offset recorded
        repo.role_renewal
            .mark_notified(created.renewal_id, 14)
            .await
            .unwrap();
        let after = repo
            .role_renewal
            .find_pending_needing_notification(ROLE_NAME, 14)
            .await
            .unwrap();
        let row = after
            .iter()
            .find(|p| p.renewal_id == created.renewal_id)
            .unwrap();
        assert_eq!(row.notified_days, vec![14]);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn overdue_sweep_honors_grace_period() {
        let (repo, db_url) = setup_test_db().await;
        let renewal = seed(&repo).await;
        sqlx::query("UPDATE Role SET grace_period_days = 14 WHERE name = $1")
            .bind(ROLE_NAME)
            .execute(&repo.member.pool)
            .await
            .unwrap();

        let today = Utc::now().date_naive();
        // (valid_from offset, valid_until offset, expected overdue)
        let cases = [(-400, -14, false), (-500, -15, true)];
        let mut ids = Vec::new();
        for (from, until, _) in cases {
            let valid_from = today + Duration::days(from);
            let valid_until = today + Duration::days(until);
            repo.role
                .create_role_member(&user_id(), ROLE_NAME, valid_from, Some(valid_until))
                .await
                .unwrap();
            let created = repo
                .role_renewal
                .create(&RoleRenewal {
                    renewal_id: Uuid::new_v4(),
                    old_valid_from: valid_from,
                    old_valid_until: valid_until,
                    new_valid_from: valid_until + Duration::days(1),
                    new_valid_until: valid_until + Duration::days(366),
                    ..renewal.clone()
                })
                .await
                .unwrap();
            ids.push(created.renewal_id);
        }

        let overdue = repo.role_renewal.find_overdue_pending().await.unwrap();
        for ((_, until, expected), id) in cases.iter().zip(&ids) {
            assert_eq!(
                overdue.iter().any(|r| r.renewal_id == *id),
                *expected,
                "renewal expiring at {until} days"
            );
        }

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn paid_renewal_does_not_requalify_membership() {
        let (repo, db_url) = setup_test_db().await;
        let renewal = seed(&repo).await;

        let qualifies = |rows: &[crate::application::ports::role_renewal_repository_port::RenewableExpiring]| {
            rows.iter()
                .any(|e| e.user_id == user_id() && e.role_name == ROLE_NAME)
        };

        let before = repo.role_renewal.find_expiring_renewable().await.unwrap();
        assert!(qualifies(&before));

        let created = repo.role_renewal.create(&renewal).await.unwrap();
        let with_pending = repo.role_renewal.find_expiring_renewable().await.unwrap();
        assert!(!qualifies(&with_pending));

        repo.role_renewal
            .mark_paid(created.renewal_id, "pi_test")
            .await
            .unwrap();
        let with_paid = repo.role_renewal.find_expiring_renewable().await.unwrap();
        assert!(
            !qualifies(&with_paid),
            "paid renewal must not requalify the old membership for reminders"
        );

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn expiring_horizon_covers_renewal_window_per_role() {
        let (repo, db_url) = setup_test_db().await;
        // Role has window 30 and reminder offsets [14]
        let _ = seed(&repo).await;
        let today = Utc::now().date_naive();

        // Inside the window though beyond the largest reminder offset
        repo.role
            .create_role_member(
                &user_id(),
                ROLE_NAME,
                today - Duration::days(100),
                Some(today + Duration::days(20)),
            )
            .await
            .unwrap();
        // Beyond the window
        repo.role
            .create_role_member(
                &user_id(),
                ROLE_NAME,
                today - Duration::days(50),
                Some(today + Duration::days(40)),
            )
            .await
            .unwrap();

        let expiring = repo.role_renewal.find_expiring_renewable().await.unwrap();
        let until_days: Vec<i64> = expiring
            .iter()
            .filter(|e| e.user_id == user_id() && e.role_name == ROLE_NAME)
            .map(|e| (e.valid_until - today).num_days())
            .collect();
        assert!(until_days.contains(&10));
        assert!(until_days.contains(&20));
        assert!(!until_days.contains(&40));

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn second_pending_renewal_for_same_membership_is_rejected() {
        let (repo, db_url) = setup_test_db().await;
        let renewal = seed(&repo).await;
        repo.role_renewal.create(&renewal).await.unwrap();

        let duplicate = RoleRenewal {
            renewal_id: Uuid::new_v4(),
            ..renewal.clone()
        };
        let result = repo.role_renewal.create(&duplicate).await;
        assert!(matches!(result, Err(RepositoryError::Constraint(_))));

        cleanup_test_db(repo.member.pool, &db_url).await;
    }
}
