#[cfg(test)]
mod test_member {
    use uuid::Uuid;

    use crate::application::ports::member_repository_port::{
        MemberRepositoryPort, MembersWithRolesParams,
    };
    use crate::domain::{Email, NewPerson, PersonId};
    use crate::infrastructure::repositories::tests::{cleanup_test_db, setup_test_db};

    fn _get_new_member() -> NewPerson {
        let user_id = Uuid::new_v4();
        NewPerson {
            id: PersonId(user_id),
            first_name: "Testi".to_string(),
            last_name: "Käyttäjä".to_string(),
            home_municipality: "Helsinki".to_string(),
            email: Email::new("john@example.com".to_string()).unwrap(),
            has_accepted_policies: true,
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
                "Uusi",
                &member_to_update.last_name,
                &member_to_update.home_municipality,
                member_to_update.has_accepted_policies,
            )
            .await
            .unwrap();

        assert!(updated_member.first_name == "Uusi");

        cleanup_test_db(repo.member.pool, &db_url).await;
    }
}
