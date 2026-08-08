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
            renewal_window_days: 30,
            grace_period_days: 0,
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
                renewal_window_days: 30,
                grace_period_days: 0,
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

    #[tokio::test]
    async fn renewal_due_respects_window_and_grace() {
        let (repo, db_url) = setup_test_db().await;
        let user = _get_user_id();
        let today = chrono::Utc::now().date_naive();

        let renewal_defaults = Role {
            name: RoleName(String::new()),
            color: None,
            description: None,
            renewable: true,
            renewal_payment_link: Some("https://pay".to_string()),
            renewal_period_months: Some(12),
            renewal_email_template: None,
            renewal_notification_days: vec![30, 7, 1],
            renewal_window_days: 30,
            grace_period_days: 0,
        };

        // Wide window: expiry 35 days out is already due
        repo.role
            .create(&Role {
                name: RoleName("wide-window".to_string()),
                renewal_window_days: 40,
                grace_period_days: 30,
                ..renewal_defaults.clone()
            })
            .await
            .unwrap();
        repo.role
            .create_role_member(
                &user,
                "wide-window",
                today - chrono::Duration::days(330),
                Some(today + chrono::Duration::days(35)),
            )
            .await
            .unwrap();

        // Default window, expired 10 days ago, no grace: not due
        repo.role
            .create(&Role {
                name: RoleName("no-grace".to_string()),
                ..renewal_defaults.clone()
            })
            .await
            .unwrap();
        repo.role
            .create_role_member(
                &user,
                "no-grace",
                today - chrono::Duration::days(375),
                Some(today - chrono::Duration::days(10)),
            )
            .await
            .unwrap();

        // Inside the window but renewal is not configured: not due
        repo.role
            .create(&Role {
                name: RoleName("unconfigured".to_string()),
                renewal_payment_link: None,
                ..renewal_defaults.clone()
            })
            .await
            .unwrap();
        repo.role
            .create_role_member(
                &user,
                "unconfigured",
                today - chrono::Duration::days(355),
                Some(today + chrono::Duration::days(10)),
            )
            .await
            .unwrap();

        let memberships = repo.role.fetch_roles_by_member(&user).await.unwrap();

        let wide = memberships
            .iter()
            .find(|m| m.role_name.0 == "wide-window")
            .unwrap();
        assert!(wide.renewal_due);
        assert_eq!(
            wide.renewal_deadline,
            Some(today + chrono::Duration::days(35 + 30))
        );

        let no_grace = memberships
            .iter()
            .find(|m| m.role_name.0 == "no-grace")
            .unwrap();
        assert!(!no_grace.renewal_due);

        let unconfigured = memberships
            .iter()
            .find(|m| m.role_name.0 == "unconfigured")
            .unwrap();
        assert!(!unconfigured.renewal_due);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn renewal_due_boundaries_are_inclusive() {
        let (repo, db_url) = setup_test_db().await;
        let user = _get_user_id();
        let today = chrono::Utc::now().date_naive();

        let renewal_defaults = Role {
            name: RoleName(String::new()),
            color: None,
            description: None,
            renewable: true,
            renewal_payment_link: Some("https://pay".to_string()),
            renewal_period_months: Some(12),
            renewal_email_template: None,
            renewal_notification_days: vec![30, 7, 1],
            renewal_window_days: 30,
            grace_period_days: 14,
        };

        // (role, days from today to valid_until, expected renewal_due)
        let cases = [
            ("opens-today", 30, true),
            ("opens-tomorrow", 31, false),
            ("last-grace-day", -14, true),
            ("past-grace", -15, false),
        ];
        for (name, offset, _) in cases {
            repo.role
                .create(&Role {
                    name: RoleName(name.to_string()),
                    ..renewal_defaults.clone()
                })
                .await
                .unwrap();
            repo.role
                .create_role_member(
                    &user,
                    name,
                    today + chrono::Duration::days(offset - 365),
                    Some(today + chrono::Duration::days(offset)),
                )
                .await
                .unwrap();
        }

        let memberships = repo.role.fetch_roles_by_member(&user).await.unwrap();
        for (name, _, expected) in cases {
            let m = memberships
                .iter()
                .find(|m| m.role_name.0 == name)
                .unwrap();
            assert_eq!(m.renewal_due, expected, "role {name}");
        }

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn replace_and_fetch_renewal_prompts() {
        let (repo, db_url) = setup_test_db().await;

        repo.role
            .create(&Role {
                name: RoleName("prompt-role".to_string()),
                color: None,
                description: None,
                renewable: true,
                renewal_payment_link: None,
                renewal_period_months: Some(12),
                renewal_email_template: None,
                renewal_notification_days: vec![30, 7, 1],
                renewal_window_days: 30,
                grace_period_days: 0,
            })
            .await
            .unwrap();

        let fi = crate::domain::RenewalPrompt {
            locale: "fi".to_string(),
            title: "Jäsenmaksu vuodelle {year}".to_string(),
            body: "Et ole maksanut jäsenmaksua vuodelle {year}.".to_string(),
            button_label: "Maksa jäsenmaksu".to_string(),
        };
        let en = crate::domain::RenewalPrompt {
            locale: "en".to_string(),
            title: "Membership fee for {year}".to_string(),
            body: "You have not paid the membership fee for {year}.".to_string(),
            button_label: "Pay membership fee".to_string(),
        };

        // A second role with its own prompt row must be untouched by replaces
        // targeting the first role.
        repo.role
            .create(&Role {
                name: RoleName("other-role".to_string()),
                color: None,
                description: None,
                renewable: true,
                renewal_payment_link: None,
                renewal_period_months: Some(12),
                renewal_email_template: None,
                renewal_notification_days: vec![30, 7, 1],
                renewal_window_days: 30,
                grace_period_days: 0,
            })
            .await
            .unwrap();
        let other = crate::domain::RenewalPrompt {
            locale: "fi".to_string(),
            title: "Toisen roolin otsikko".to_string(),
            body: "Toisen roolin teksti".to_string(),
            button_label: "Toisen roolin nappi".to_string(),
        };
        repo.role
            .replace_renewal_prompts("other-role", &[other.clone()])
            .await
            .unwrap();

        repo.role
            .replace_renewal_prompts("prompt-role", &[fi.clone(), en.clone()])
            .await
            .unwrap();
        let fetched = repo
            .role
            .fetch_renewal_prompts("prompt-role")
            .await
            .unwrap();
        assert_eq!(fetched.len(), 2);
        assert!(fetched.contains(&fi));

        // Replace drops rows not in the new set
        repo.role
            .replace_renewal_prompts("prompt-role", &[en.clone()])
            .await
            .unwrap();
        let fetched = repo
            .role
            .fetch_renewal_prompts("prompt-role")
            .await
            .unwrap();
        assert_eq!(fetched, vec![en]);

        // Empty set clears everything
        repo.role
            .replace_renewal_prompts("prompt-role", &[])
            .await
            .unwrap();
        assert!(repo
            .role
            .fetch_renewal_prompts("prompt-role")
            .await
            .unwrap()
            .is_empty());

        // Every replace above was scoped to prompt-role
        assert_eq!(
            repo.role.fetch_renewal_prompts("other-role").await.unwrap(),
            vec![other]
        );

        cleanup_test_db(repo.member.pool, &db_url).await;
    }
}
