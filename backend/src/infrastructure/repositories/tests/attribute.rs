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

    fn create_languages(multiple: bool, allow_other: bool) -> CreateAttributeDefinition {
        CreateAttributeDefinition {
            name: languages(),
            description: None,
            allowed_values: Some(avs(&["fi", "sv", "en"])),
            default_value: None,
            sync_to_keycloak: false,
            editable_by: EditableBy::Both,
            required: false,
            multiple,
            allow_other,
        }
    }

    #[tokio::test]
    async fn definition_round_trips_multiple_and_allow_other() {
        let (repo, db_url) = setup_test_db().await;

        let created = repo
            .attribute
            .create_definition(create_languages(true, true))
            .await
            .unwrap();
        assert!(created.multiple());
        assert!(created.allow_other());

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
                    allow_other: false,
                },
            )
            .await
            .unwrap();
        assert!(!updated.multiple());
        assert!(!updated.allow_other());

        let all = repo.attribute.fetch_all_definitions().await.unwrap();
        assert!(all.contains(&updated));

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn member_values_round_trip_in_order_and_upsert_replaces_the_list() {
        let (repo, db_url) = setup_test_db().await;
        repo.attribute
            .create_definition(create_languages(true, true))
            .await
            .unwrap();

        repo.attribute
            .upsert_member_value(&user_id(), &languages(), &avs(&["fi", "en", "de"]))
            .await
            .unwrap();
        let held = repo
            .attribute
            .fetch_member_values(&user_id())
            .await
            .unwrap();
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].values, avs(&["fi", "en", "de"]));

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
            .create_definition(create_languages(true, false))
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

    #[tokio::test]
    async fn auto_default_values_lists_untouched_and_resubmitted_defaults_only() {
        let (repo, db_url) = setup_test_db().await;
        let pool = &repo.member.pool;
        // study-year (both) and pora (user) are member-filled; member-type
        // is admin-only, so its default stays.
        sqlx::raw_sql(
            r#"
            INSERT INTO AttributeDefinition (name, allowed_values, default_value, sync_to_keycloak, editable_by)
            VALUES ('study-year', NULL, '2026', false, 'both'),
                   ('pora', ARRAY['yes','no'], 'yes', false, 'user'),
                   ('member-type', NULL, 'regular', false, 'admin');
            INSERT INTO Member (user_id, email, first_name, last_name) VALUES
             ('00000000-0000-0000-0000-00000000000a','a@x.fi','Anna','Untouched'),
             ('00000000-0000-0000-0000-00000000000b','b@x.fi','Bert','Resubmit'),
             ('00000000-0000-0000-0000-00000000000c','c@x.fi','Cecilia','Changed'),
             ('00000000-0000-0000-0000-00000000000e','e@x.fi','Eero','Cleared');
            INSERT INTO MemberAttribute (user_id, attribute_name, value_list) VALUES
             ('00000000-0000-0000-0000-00000000000a','study-year',ARRAY['2026']),
             ('00000000-0000-0000-0000-00000000000a','pora',ARRAY['yes']),
             ('00000000-0000-0000-0000-00000000000a','member-type',ARRAY['regular']),
             ('00000000-0000-0000-0000-00000000000b','study-year',ARRAY['2026']),
             ('00000000-0000-0000-0000-00000000000c','study-year',ARRAY['2023']);
            INSERT INTO audit_log (action, entity_type, entity_id, details, created_at) VALUES
             ('member_attribute.set','member_attribute','00000000-0000-0000-0000-00000000000a:study-year','{"actor_kind":"system","value":"2026"}', now()-interval '5 day'),
             ('member_attribute.set','member_attribute','00000000-0000-0000-0000-00000000000a:pora','{"actor_kind":"system","value":"yes"}', now()-interval '5 day'),
             ('member_attribute.set','member_attribute','00000000-0000-0000-0000-00000000000a:member-type','{"actor_kind":"system","value":"regular"}', now()-interval '5 day'),
             ('member_attribute.set','member_attribute','00000000-0000-0000-0000-00000000000b:study-year','{"actor_kind":"system","value":"2026"}', now()-interval '4 day'),
             ('member_attribute.set','member_attribute','00000000-0000-0000-0000-00000000000b:study-year','{"actor_kind":"application_form","values":["2026"]}', now()-interval '3 day'),
             ('member_attribute.set','member_attribute','00000000-0000-0000-0000-00000000000c:study-year','{"actor_kind":"system","value":"2026"}', now()-interval '4 day'),
             ('member_attribute.set','member_attribute','00000000-0000-0000-0000-00000000000c:study-year','{"actor_kind":"self","values":["2023"]}', now()-interval '2 day'),
             ('member_attribute.set','member_attribute','00000000-0000-0000-0000-00000000000e:study-year','{"actor_kind":"system","value":"2026"}', now()-interval '4 day'),
             ('member_attribute.clear','member_attribute','00000000-0000-0000-0000-00000000000e:study-year','{"actor_kind":"self"}', now()-interval '1 day');
            "#,
        )
        .execute(pool)
        .await
        .unwrap();

        let rows = repo.attribute.fetch_auto_default_values().await.unwrap();
        let summary: Vec<(&str, &str, bool)> = rows
            .iter()
            .map(|r| (r.email.as_str(), r.attribute.as_str(), r.resubmitted))
            .collect();
        assert_eq!(
            summary,
            [
                ("a@x.fi", "pora", false),
                ("a@x.fi", "study-year", false),
                ("b@x.fi", "study-year", true),
            ]
        );
        assert_eq!(rows[1].full_name, "Anna Untouched");
        assert_eq!(rows[1].values, avs(&["2026"]));

        cleanup_test_db(repo.member.pool, &db_url).await;
    }
}
