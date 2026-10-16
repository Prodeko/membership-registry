#[cfg(test)]
mod test_member {
    use uuid::Uuid;

    use sqlx::PgPool;

    use crate::application::ports::member_repository_port::{
        AttributeFilter, MemberRepositoryPort, MembersWithRolesParams,
    };
    use crate::domain::{
        AttributeName, AttributeValue, Email, NewPerson, PersonId, UpdatePersonData,
    };
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

    const TANELI: &str = "3e1ab0ea-c56a-457f-961f-13938954bb2b";
    const JOHANNA: &str = "11880220-c535-4afe-87a2-c59e213f87f4";
    const JUHA: &str = "ffbf88de-758d-498c-b6b4-6968162f2086";

    /// Seeds `start-year` (single-valued) and `interests` (multi-valued):
    /// Taneli 2026 + {sports, music}, Johanna 2026 + {music}, Juha 2025.
    async fn seed_attributes(pool: &PgPool) {
        sqlx::query(
            "INSERT INTO AttributeDefinition (name, sync_to_keycloak, editable_by, multiple) VALUES
             ('start-year', false, 'admin', false),
             ('interests', false, 'admin', true)",
        )
        .execute(pool)
        .await
        .unwrap();
        for (user, name, values) in [
            (TANELI, "start-year", vec!["2026"]),
            (JOHANNA, "start-year", vec!["2026"]),
            (JUHA, "start-year", vec!["2025"]),
            (TANELI, "interests", vec!["sports", "music"]),
            (JOHANNA, "interests", vec!["music"]),
        ] {
            sqlx::query(
                "INSERT INTO MemberAttribute (user_id, attribute_name, value_list)
                 VALUES ($1::uuid, $2, $3)",
            )
            .bind(user)
            .bind(name)
            .bind(values)
            .execute(pool)
            .await
            .unwrap();
        }
    }

    fn filter(name: &str, values: &[&str]) -> AttributeFilter {
        AttributeFilter {
            name: AttributeName::new(name).unwrap(),
            values: values
                .iter()
                .map(|v| AttributeValue::new(*v).unwrap())
                .collect(),
        }
    }

    /// Member ids (sorted) and the count for `filters`.
    async fn matching(
        repo: &crate::infrastructure::repositories::PostgresRepo,
        filters: Vec<AttributeFilter>,
    ) -> (Vec<String>, i64) {
        let mut ids: Vec<String> = repo
            .member
            .fetch_members_with_roles(MembersWithRolesParams {
                attributes: filters.clone(),
                ..Default::default()
            })
            .await
            .unwrap()
            .into_iter()
            .map(|m| m.person.id.0.to_string())
            .collect();
        ids.sort();
        let count = repo
            .member
            .count_members_with_roles(MembersWithRolesParams {
                attributes: filters,
                ..Default::default()
            })
            .await
            .unwrap();
        (ids, count)
    }

    fn sorted(ids: &[&str]) -> Vec<String> {
        let mut v: Vec<String> = ids.iter().map(|s| s.to_string()).collect();
        v.sort();
        v
    }

    #[tokio::test]
    async fn attribute_filter_matches_value() {
        let (repo, db_url) = setup_test_db().await;
        seed_attributes(&repo.member.pool).await;

        let (ids, count) = matching(&repo, vec![filter("start-year", &["2026"])]).await;
        assert_eq!(ids, sorted(&[TANELI, JOHANNA]));
        assert_eq!(count, 2);

        // Any of several values.
        let (ids, count) = matching(&repo, vec![filter("start-year", &["2025", "2026"])]).await;
        assert_eq!(ids, sorted(&[TANELI, JOHANNA, JUHA]));
        assert_eq!(count, 3);

        // A value nobody has.
        let (ids, count) = matching(&repo, vec![filter("start-year", &["2020"])]).await;
        assert!(ids.is_empty());
        assert_eq!(count, 0);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn attribute_filters_combine_and_match_any_value() {
        let (repo, db_url) = setup_test_db().await;
        seed_attributes(&repo.member.pool).await;

        // Multi-valued attribute: any overlap matches.
        let (ids, _) = matching(&repo, vec![filter("interests", &["sports"])]).await;
        assert_eq!(ids, sorted(&[TANELI]));

        // Filters on different attributes must all match.
        let (ids, count) = matching(
            &repo,
            vec![
                filter("start-year", &["2026"]),
                filter("interests", &["music"]),
            ],
        )
        .await;
        assert_eq!(ids, sorted(&[TANELI, JOHANNA]));
        assert_eq!(count, 2);
        let (ids, _) = matching(
            &repo,
            vec![
                filter("start-year", &["2025"]),
                filter("interests", &["music"]),
            ],
        )
        .await;
        assert!(ids.is_empty());

        // No values: anyone holding the attribute.
        let (ids, count) = matching(&repo, vec![filter("interests", &[])]).await;
        assert_eq!(ids, sorted(&[TANELI, JOHANNA]));
        assert_eq!(count, 2);

        // No filters: everyone, as before.
        let (_, count) = matching(&repo, vec![]).await;
        assert_eq!(count, 10);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn fetch_by_email_is_case_insensitive() {
        let (repo, db_url) = setup_test_db().await;
        let id = Uuid::new_v4();
        repo.member
            .create(NewPerson {
                id: PersonId(id),
                email: Email::new_unchecked("Mixed@Case.com".into()),
                first_name: "A".into(),
                last_name: "B".into(),
                home_municipality: None,
                email_notifications: true,
                language: "fi".into(),
            })
            .await
            .unwrap();

        let found = repo.member.fetch_by_email("mixed@case.com").await.unwrap();
        assert_eq!(found.unwrap().id.0, id);
        assert!(repo
            .member
            .fetch_by_email("nobody@x.com")
            .await
            .unwrap()
            .is_none());

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
