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
    async fn notification_query_honors_arbitrary_offset() {
        let (repo, db_url) = setup_test_db().await;
        let renewal = seed(&repo).await;
        let created = repo.role_renewal.create(&renewal).await.unwrap();

        // Expiry is 10 days out: due at the 14-day offset, not at 7
        let due = repo
            .role_renewal
            .find_pending_needing_notification(ROLE_NAME, 14)
            .await
            .unwrap();
        assert!(due.iter().any(|p| p.renewal_id == created.renewal_id));

        let not_due = repo
            .role_renewal
            .find_pending_needing_notification(ROLE_NAME, 7)
            .await
            .unwrap();
        assert!(not_due.iter().all(|p| p.renewal_id != created.renewal_id));

        repo.role_renewal
            .mark_notified(created.renewal_id, 14)
            .await
            .unwrap();
        let after = repo
            .role_renewal
            .find_pending_needing_notification(ROLE_NAME, 14)
            .await
            .unwrap();
        assert!(after.iter().all(|p| p.renewal_id != created.renewal_id));

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
