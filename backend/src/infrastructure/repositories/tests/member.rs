#[cfg(test)]
mod test_member {
    use uuid::Uuid;

    use crate::application::ports::member_repository_port::{
        MemberRepositoryPort, MembersWithRolesParams,
    };
    use crate::domain::{Email, NewPerson, PersonId, UpdatePersonData};
    use crate::infrastructure::repositories::tests::{cleanup_test_db, setup_test_db};

    fn _get_new_member() -> NewPerson {
        let user_id = Uuid::new_v4();
        NewPerson {
            id: PersonId(user_id),
            first_name: "Testi".to_string(),
            last_name: "Käyttäjä".to_string(),
            home_municipality: Some("Helsinki".to_string()),
            email: Email::new("john@example.com".to_string()).unwrap(),
            email_notifications: true,
            language: "fi".to_string(),
        }
    }

    #[tokio::test]
    async fn test_create_member() {
        let (repo, db_url) = setup_test_db().await;
        let member_to_add = _get_new_member();
        let expected_id = member_to_add.id.0;
        let member = repo.member.create(member_to_add).await;

        assert!(member.unwrap().id.0 == expected_id);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_get_member() {
        let (repo, db_url) = setup_test_db().await;
        let user_id = Uuid::parse_str("3e1ab0ea-c56a-457f-961f-13938954bb2b").unwrap();

        let member_from_db = repo.member.fetch_one(user_id).await.unwrap();

        assert!(member_from_db.id.0 == user_id);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_get_all_members() {
        let (repo, db_url) = setup_test_db().await;

        let members = repo.member.fetch_all().await.unwrap();

        assert!(members.len() == 10);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_get_members_with_roles_without_params() {
        let (repo, db_url) = setup_test_db().await;

        let members = repo
            .member
            .fetch_members_with_roles(MembersWithRolesParams {
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(members.len() == 10);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_get_members_with_roles_offsets() {
        let (repo, db_url) = setup_test_db().await;

        let members = repo
            .member
            .fetch_members_with_roles(MembersWithRolesParams {
                page_size: Some(5),
                offset: Some(1),
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(members.len() == 5);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_get_members_with_roles_single_role_filter() {
        let (repo, db_url) = setup_test_db().await;

        let members = repo
            .member
            .fetch_members_with_roles(MembersWithRolesParams {
                roles: Some(vec!["prodeko-external-member".to_string()]),
                ..Default::default()
            })
            .await
            .unwrap();

        println!("{:?}", members.len());

        assert!(members.len() == 3);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_get_members_with_roles_multiple_role_filter() {
        let (repo, db_url) = setup_test_db().await;

        let members = repo
            .member
            .fetch_members_with_roles(MembersWithRolesParams {
                roles: Some(vec![
                    "prodeko-external-member".to_string(),
                    "prodeko-board".to_string(),
                ]),
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(members.len() == 4);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_get_members_with_roles_search() {
        let (repo, db_url) = setup_test_db().await;

        let members = repo
            .member
            .fetch_members_with_roles(MembersWithRolesParams {
                search: Some("Taneli".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(members.len() == 1);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_get_members_with_roles_order_by() {
        let (repo, db_url) = setup_test_db().await;

        let members = repo
            .member
            .fetch_members_with_roles(MembersWithRolesParams {
                order_by: Some("first_name".to_string()),
                order_desc: Some(true),
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(members[0].person.first_name == "Zanteri");

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_get_members_with_roles_valid_from() {
        let (repo, db_url) = setup_test_db().await;

        let members = repo
            .member
            .fetch_members_with_roles(MembersWithRolesParams {
                roles: Some(vec!["prodeko-full-member".to_string()]),
                valid_from: chrono::NaiveDate::from_ymd_opt(2020, 1, 1),
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(
            members[0].person.id.0
                == Uuid::parse_str("b20e5370-f0ae-4c8f-bdf0-5743b8f85c7a").unwrap()
        );

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_get_members_with_roles_date_range() {
        let (repo, db_url) = setup_test_db().await;

        let members = repo
            .member
            .fetch_members_with_roles(MembersWithRolesParams {
                roles: Some(vec!["prodeko-board".to_string()]),
                valid_from: chrono::NaiveDate::from_ymd_opt(2021, 1, 1),
                valid_until: chrono::NaiveDate::from_ymd_opt(2021, 12, 31),
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(members.len() == 1);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    /// With a date filter but NO role filter, role_names must only contain
    /// roles active on that date — the bug was that the date filter was
    /// skipped entirely when roles was None.
    #[tokio::test]
    async fn test_role_names_date_filter_applies_without_role_filter() {
        let (repo, db_url) = setup_test_db().await;
        let today = chrono::Utc::now().date_naive();

        let members = repo
            .member
            .fetch_members_with_roles(MembersWithRolesParams {
                valid_from: Some(today),
                valid_until: Some(today),
                ..Default::default()
            })
            .await
            .unwrap();

        // All 10 members still appear — no member is hidden by the date filter.
        assert_eq!(members.len(), 10);

        let antti = members
            .iter()
            .find(|m| {
                m.person.id.0 == Uuid::parse_str("9707582e-c149-45a7-bae1-4b0f4de4b06f").unwrap()
            })
            .unwrap();
        // prodeko-external-member and root-users both run until 2999 — active.
        let mut active: Vec<&str> = antti.role_names.iter().map(String::as_str).collect();
        active.sort();
        assert_eq!(active, vec!["prodeko-external-member", "root-users"]);

        // Taneli Mäkinen only has roles that expired in 2022-2023.
        let taneli = members
            .iter()
            .find(|m| {
                m.person.id.0 == Uuid::parse_str("3e1ab0ea-c56a-457f-961f-13938954bb2b").unwrap()
            })
            .unwrap();
        assert!(
            taneli.role_names.is_empty(),
            "expired roles must not appear in role_names"
        );

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    /// Role filter combined with date filter should only return members who
    /// have that role active on the given date.
    #[tokio::test]
    async fn test_role_names_date_filter_with_role_filter() {
        let (repo, db_url) = setup_test_db().await;
        let today = chrono::Utc::now().date_naive();

        let members = repo
            .member
            .fetch_members_with_roles(MembersWithRolesParams {
                roles: Some(vec!["prodeko-external-member".to_string()]),
                valid_from: Some(today),
                valid_until: Some(today),
                ..Default::default()
            })
            .await
            .unwrap();

        // Only Antti Nyberg has prodeko-external-member active today (2025–2999).
        assert_eq!(members.len(), 1);
        assert_eq!(
            members[0].person.id.0,
            Uuid::parse_str("9707582e-c149-45a7-bae1-4b0f4de4b06f").unwrap()
        );

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_delete_member() {
        let (repo, db_url) = setup_test_db().await;
        let user_id = Uuid::parse_str("3e1ab0ea-c56a-457f-961f-13938954bb2b").unwrap();

        let result = repo.member.delete(user_id).await;

        assert!(result.is_ok());

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_update_member() {
        let (repo, db_url) = setup_test_db().await;
        let user_id = Uuid::parse_str("3e1ab0ea-c56a-457f-961f-13938954bb2b").unwrap();

        let member_to_update = repo.member.fetch_one(user_id).await.unwrap();

        let updated_member = repo
            .member
            .update(
                user_id,
                &UpdatePersonData {
                    first_name: "Uusi".to_string(),
                    last_name: member_to_update.last_name.clone(),
                    home_municipality: member_to_update.home_municipality.clone(),
                    email_notifications: member_to_update.email_notifications,
                    language: member_to_update.language.clone(),
                    email: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(updated_member.first_name, "Uusi");
        assert_eq!(
            updated_member.email.as_str(),
            member_to_update.email.as_str(),
            "email unchanged when None"
        );

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_update_member_email() {
        let (repo, db_url) = setup_test_db().await;
        let user_id = Uuid::parse_str("3e1ab0ea-c56a-457f-961f-13938954bb2b").unwrap();

        let member_to_update = repo.member.fetch_one(user_id).await.unwrap();

        let updated_member = repo
            .member
            .update(
                user_id,
                &UpdatePersonData {
                    first_name: member_to_update.first_name.clone(),
                    last_name: member_to_update.last_name.clone(),
                    home_municipality: member_to_update.home_municipality.clone(),
                    email_notifications: member_to_update.email_notifications,
                    language: member_to_update.language.clone(),
                    email: Some("new-email@example.com".to_string()),
                },
            )
            .await
            .unwrap();

        assert_eq!(updated_member.email.as_str(), "new-email@example.com");

        cleanup_test_db(repo.member.pool, &db_url).await;
    }
}
