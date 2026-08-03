#[cfg(test)]
mod test_role {
    use chrono::NaiveDate;
    use uuid::Uuid;

    use crate::application::ports::role_repository_port::{
        RoleRepositoryPort, RolesWithStatsParams,
    };
    use crate::domain::{Role, RoleName};
    use crate::infrastructure::repositories::tests::{cleanup_test_db, setup_test_db};

    const ROLE_NAME: &str = "prodeko-external-member";

    fn _get_user_id() -> Uuid {
        Uuid::parse_str("9707582e-c149-45a7-bae1-4b0f4de4b06f").unwrap()
    }

    #[tokio::test]
    async fn test_create_role() {
        let (repo, db_url) = setup_test_db().await;

        let name = "TestRole".to_string();
        let role_to_add = Role {
            name: RoleName(name.clone()),
            color: None,
            description: None,
            renewable: false,
            renewal_payment_link: None,
            renewal_period_months: None,
            renewal_email_template: None,
            renewal_notification_days: vec![30, 7, 1],
        };

        let role = repo.role.create(&role_to_add).await;

        assert!(role.unwrap().name.0 == name);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_delete_role_with_members() {
        let (repo, db_url) = setup_test_db().await;

        let role = repo.role.delete(ROLE_NAME).await;

        // Role has members, so it should not be deleted
        assert!(role.is_err());

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_delete_role_without_members() {
        let (repo, db_url) = setup_test_db().await;

        let _created = repo
            .role
            .create(&Role {
                name: RoleName("test-role".to_string()),
                color: None,
                description: None,
                renewable: false,
                renewal_payment_link: None,
                renewal_period_months: None,
                renewal_email_template: None,
                renewal_notification_days: vec![30, 7, 1],
            })
            .await;

        let role = repo.role.delete("test-role").await;

        // Role has no members, so it should be deleted
        assert!(role.is_ok());

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_create_role_member() {
        let (repo, db_url) = setup_test_db().await;

        let role_member = repo
            .role
            .create_role_member(
                &_get_user_id(),
                ROLE_NAME,
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2025, 1, 1),
            )
            .await;

        assert!(role_member.is_ok());

        let role_members = repo.role.fetch_roles_by_member(&_get_user_id()).await;

        assert!(role_members.unwrap().len() == 6);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn upsert_role_member_overwrites_valid_until() {
        let (repo, db_url) = setup_test_db().await;

        let uid = _get_user_id();
        let vf = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();

        repo.role
            .upsert_role_member(&uid, ROLE_NAME, vf, None)
            .await
            .unwrap();
        repo.role
            .upsert_role_member(
                &uid,
                ROLE_NAME,
                vf,
                Some(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()),
            )
            .await
            .unwrap();

        let roles = repo.role.fetch_roles_by_member(&uid).await.unwrap();
        let m = roles
            .iter()
            .find(|r| r.role_name.0 == ROLE_NAME && r.valid_from == vf)
            .unwrap();
        assert_eq!(
            m.valid_until,
            Some(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap())
        );

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_fetch_role_by_member() {
        let (repo, db_url) = setup_test_db().await;

        let role_members = repo.role.fetch_roles_by_member(&_get_user_id()).await;

        assert!(role_members.unwrap().len() == 5);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_fetch_roles_with_stats() {
        let (repo, db_url) = setup_test_db().await;

        let roles = repo
            .role
            .fetch_roles_with_stats(RolesWithStatsParams::default())
            .await;
        let roles = roles.unwrap();
        assert!(roles.len() == 8);
        assert!(roles[0].name.0 == "pora-member");
        assert!(roles[0].member_count.unwrap() == 2);
        assert!(roles[0].active_member_count.unwrap() == 1);
    }

    #[tokio::test]
    async fn test_fetch_roles_with_stats_pagination() {
        let (repo, db_url) = setup_test_db().await;

        let roles = repo
            .role
            .fetch_roles_with_stats(RolesWithStatsParams {
                page_size: Some(2),
                offset: Some(1),
                ..Default::default()
            })
            .await;
        let roles = roles.unwrap();
        assert!(roles.len() == 2);
    }

    #[tokio::test]
    async fn test_fetch_roles_with_stats_search() {
        let (repo, db_url) = setup_test_db().await;

        let roles = repo
            .role
            .fetch_roles_with_stats(RolesWithStatsParams {
                search: Some("prodeko".to_string()),
                ..Default::default()
            })
            .await;
        let roles = roles.unwrap();
        assert!(roles.len() == 6);
    }

    #[tokio::test]
    async fn test_fetch_roles_with_stats_order() {
        let (repo, db_url) = setup_test_db().await;

        let roles = repo
            .role
            .fetch_roles_with_stats(RolesWithStatsParams {
                order_by: Some("Description".to_string()),
                order_desc: Some(false),
                ..Default::default()
            })
            .await;
        let roles = roles.unwrap();
        assert!(roles.len() == 8);
        assert!(roles[0].name.0 == "root-users");
    }
}
