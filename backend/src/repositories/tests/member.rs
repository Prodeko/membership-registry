#[cfg(test)]
mod test_member {
    use ory_client::models::Pagination;
    use uuid::Uuid;

    use crate::repositories::{
        member::{Member, MembersWithRolesParams, NewMember}, tests::{cleanup_test_db, setup_test_db}, PostgresRepo
    };

    fn _get_new_member() -> NewMember {
        let user_id = Uuid::new_v4();
        NewMember {
            user_id: user_id,
            first_name: "Testi".to_string(),
            last_name: "Käyttäjä".to_string(),
            home_municipality: "Helsinki".to_string(),
            email: "john@example.com".to_string(),
            has_accepted_policies: true,
        }
    }

    #[tokio::test]
    async fn test_create_member() {
        let (repo, db_url) = setup_test_db().await;
        let member_to_add = _get_new_member();
        let member = repo.member.create(member_to_add.clone()).await;

        assert!(member.unwrap().user_id == member_to_add.user_id);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_get_member() {
        let (repo, db_url) = setup_test_db().await;
        let user_id = Uuid::parse_str("3e1ab0ea-c56a-457f-961f-13938954bb2b").unwrap();

        let member_from_db = repo.member.fetch_one(user_id).await.unwrap();

        assert!(member_from_db.user_id == user_id);

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

        let members = repo.member.fetch_members_with_roles(
            MembersWithRolesParams {
              ..Default::default()
            }
        ).await.unwrap();

        assert!(members.len() == 10);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_get_members_with_roles_offsets() {
        let (repo, db_url) = setup_test_db().await;

        let members = repo.member.fetch_members_with_roles(
            MembersWithRolesParams {
              page_size: Some(5),
              offset: Some(1),
              ..Default::default()
            },
        ).await.unwrap();

        assert!(members.len() == 5);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_get_members_with_roles_single_role_filter() {
        let (repo, db_url) = setup_test_db().await;

        let members = repo.member.fetch_members_with_roles(
          MembersWithRolesParams {
          roles: Some(vec!["prodeko-external-member".to_string()]),
          ..Default::default()
        }
        ).await.unwrap();

        println!("{:?}", members);

        assert!(members.len() == 3);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn test_get_members_with_roles_multiple_role_filter() {
        let (repo, db_url) = setup_test_db().await;

        let members = repo.member.fetch_members_with_roles(
          MembersWithRolesParams {
          roles: Some(vec!["prodeko-external-member".to_string(), "prodeko-board".to_string()]),
          ..Default::default()
        }
        ).await.unwrap();

        assert!(members.len() == 4);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }
}
