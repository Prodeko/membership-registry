#[cfg(test)]
mod test_attribute {
    use uuid::Uuid;

    use crate::application::ports::attribute_repository_port::{
        AttributeRepositoryPort, CreateAttributeDefinition, UpdateAttributeDefinition,
    };
    use crate::domain::{AttributeName, AttributeValue, EditableBy, PersonId};
    use crate::infrastructure::repositories::tests::{cleanup_test_db, setup_test_db};

    fn user_id() -> PersonId {
        PersonId(Uuid::parse_str("9707582e-c149-45a7-bae1-4b0f4de4b06f").unwrap())
    }

    fn avs(vs: &[&str]) -> Vec<AttributeValue> {
        vs.iter()
            .map(|v| AttributeValue::new(*v).unwrap())
            .collect()
    }

    fn languages() -> AttributeName {
        AttributeName::new("languages").unwrap()
    }

    fn create_languages(multiple: bool) -> CreateAttributeDefinition {
        CreateAttributeDefinition {
            name: languages(),
            description: None,
            allowed_values: Some(avs(&["fi", "sv", "en"])),
            default_value: None,
            sync_to_keycloak: false,
            editable_by: EditableBy::Both,
            required: false,
            multiple,
        }
    }

    #[tokio::test]
    async fn definition_round_trips_multiple() {
        let (repo, db_url) = setup_test_db().await;

        let created = repo
            .attribute
            .create_definition(create_languages(true))
            .await
            .unwrap();
        assert!(created.multiple());

        let fetched = repo
            .attribute
            .fetch_definition(&languages())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched, created);

        let updated = repo
            .attribute
            .update_definition(
                &languages(),
                UpdateAttributeDefinition {
                    description: None,
                    allowed_values: Some(avs(&["fi", "sv", "en"])),
                    default_value: None,
                    sync_to_keycloak: false,
                    editable_by: EditableBy::Both,
                    required: false,
                    multiple: false,
                },
            )
            .await
            .unwrap();
        assert!(!updated.multiple());

        let all = repo.attribute.fetch_all_definitions().await.unwrap();
        assert!(all.contains(&updated));

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn member_values_round_trip_in_order_and_upsert_replaces_the_list() {
        let (repo, db_url) = setup_test_db().await;
        repo.attribute
            .create_definition(create_languages(true))
            .await
            .unwrap();

        repo.attribute
            .upsert_member_value(&user_id(), &languages(), &avs(&["fi", "en", "sv"]))
            .await
            .unwrap();
        let held = repo
            .attribute
            .fetch_member_values(&user_id())
            .await
            .unwrap();
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].values, avs(&["fi", "en", "sv"]));

        repo.attribute
            .upsert_member_value(&user_id(), &languages(), &avs(&["sv"]))
            .await
            .unwrap();
        let all = repo
            .attribute
            .fetch_all_values_for(&languages())
            .await
            .unwrap();
        assert_eq!(all, vec![(user_id(), avs(&["sv"]))]);

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn table_rejects_empty_value_lists_and_empty_strings() {
        let (repo, db_url) = setup_test_db().await;
        repo.attribute
            .create_definition(create_languages(true))
            .await
            .unwrap();

        for bad in [vec![], vec!["fi".to_string(), String::new()]] {
            let res = sqlx::query(
                "INSERT INTO MemberAttribute (user_id, attribute_name, value_list)
                 VALUES ($1, 'languages', $2)",
            )
            .bind(user_id().0)
            .bind(&bad)
            .execute(&repo.member.pool)
            .await;
            assert!(res.is_err(), "value_list {bad:?} should be rejected");
        }

        cleanup_test_db(repo.member.pool, &db_url).await;
    }
}
