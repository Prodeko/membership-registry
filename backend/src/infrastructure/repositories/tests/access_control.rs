#[cfg(test)]
mod test_access_control {
    use chrono::NaiveDate;
    use uuid::Uuid;

    use crate::application::ports::access_control_port::{
        AccessControlRepositoryPort, RecordOutcome, StatusChangeOutcome,
    };
    use crate::domain::{AccessDirection, AccessEventStatus, AccessMethod, NewAccessEvent};
    use crate::infrastructure::repositories::tests::{cleanup_test_db, setup_test_db};
    use crate::infrastructure::repositories::PostgresRepo;

    // Seeded members (test_data.sql).
    const TANELI: &str = "3e1ab0ea-c56a-457f-961f-13938954bb2b";
    const JOHANNA: &str = "11880220-c535-4afe-87a2-c59e213f87f4";

    fn id(s: &str) -> Uuid {
        Uuid::parse_str(s).unwrap()
    }

    async fn running_event(repo: &PostgresRepo) -> Uuid {
        let event = repo
            .access_control
            .create_event(&NewAccessEvent {
                name: "Syysvaalikokous".to_string(),
                event_date: NaiveDate::from_ymd_opt(2026, 11, 20).unwrap(),
                allowed_roles: vec!["prodeko-external-member".to_string()],
            })
            .await
            .unwrap();
        assert_eq!(event.status, AccessEventStatus::NotStarted);
        let changed = repo
            .access_control
            .change_status(event.id, AccessEventStatus::Running, None)
            .await
            .unwrap();
        assert_eq!(changed, StatusChangeOutcome::Changed);
        event.id
    }

    async fn record(
        repo: &PostgresRepo,
        event_id: Uuid,
        user: &str,
        direction: AccessDirection,
    ) -> RecordOutcome {
        repo.access_control
            .record_entry(
                event_id,
                id(user),
                direction,
                AccessMethod::Qr,
                Some(id(JOHANNA)),
            )
            .await
            .unwrap()
    }

    async fn present_ids(repo: &PostgresRepo, event_id: Uuid) -> Vec<Uuid> {
        let mut ids: Vec<Uuid> = repo
            .access_control
            .present(event_id)
            .await
            .unwrap()
            .into_iter()
            .map(|m| m.user_id)
            .collect();
        ids.sort();
        ids
    }

    #[tokio::test]
    async fn entries_follow_in_out_and_reentry() {
        let (repo, db_url) = setup_test_db().await;
        let event_id = running_event(&repo).await;

        assert_eq!(
            record(&repo, event_id, TANELI, AccessDirection::Out).await,
            RecordOutcome::NotInside
        );
        assert_eq!(
            record(&repo, event_id, TANELI, AccessDirection::In).await,
            RecordOutcome::Recorded
        );
        assert!(matches!(
            record(&repo, event_id, TANELI, AccessDirection::In).await,
            RecordOutcome::AlreadyInside { .. }
        ));
        assert_eq!(present_ids(&repo, event_id).await, vec![id(TANELI)]);

        assert_eq!(
            record(&repo, event_id, TANELI, AccessDirection::Out).await,
            RecordOutcome::Recorded
        );
        assert!(present_ids(&repo, event_id).await.is_empty());

        // Back in after a break.
        assert_eq!(
            record(&repo, event_id, TANELI, AccessDirection::In).await,
            RecordOutcome::Recorded
        );
        assert_eq!(present_ids(&repo, event_id).await, vec![id(TANELI)]);

        let entries = repo.access_control.entries(event_id).await.unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].member_name, "Taneli Mäkinen");
        assert_eq!(
            entries[0].recorded_by_name.as_deref(),
            Some("Johanna Kettunen")
        );

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn pausing_keeps_people_inside_and_ending_checks_them_out() {
        let (repo, db_url) = setup_test_db().await;
        let event_id = running_event(&repo).await;
        record(&repo, event_id, TANELI, AccessDirection::In).await;
        record(&repo, event_id, JOHANNA, AccessDirection::In).await;

        repo.access_control
            .change_status(event_id, AccessEventStatus::Paused, None)
            .await
            .unwrap();
        assert_eq!(present_ids(&repo, event_id).await.len(), 2);
        // The door still works during the break.
        assert_eq!(
            record(&repo, event_id, JOHANNA, AccessDirection::Out).await,
            RecordOutcome::Recorded
        );

        repo.access_control
            .change_status(event_id, AccessEventStatus::Ended, None)
            .await
            .unwrap();
        assert!(present_ids(&repo, event_id).await.is_empty());
        let entries = repo.access_control.entries(event_id).await.unwrap();
        let auto: Vec<_> = entries
            .iter()
            .filter(|e| e.method == AccessMethod::Auto)
            .collect();
        assert_eq!(auto.len(), 1);
        assert_eq!(auto[0].user_id, id(TANELI));
        assert_eq!(auto[0].direction, AccessDirection::Out);

        // An ended event takes no entries and can't be reopened.
        assert_eq!(
            record(&repo, event_id, TANELI, AccessDirection::In).await,
            RecordOutcome::EventNotActive
        );
        assert_eq!(
            repo.access_control
                .change_status(event_id, AccessEventStatus::Running, None)
                .await
                .unwrap(),
            StatusChangeOutcome::NotAllowed {
                current: AccessEventStatus::Ended
            }
        );

        let statuses: Vec<_> = repo
            .access_control
            .status_changes(event_id)
            .await
            .unwrap()
            .into_iter()
            .map(|c| c.status)
            .collect();
        assert_eq!(
            statuses,
            [
                AccessEventStatus::Running,
                AccessEventStatus::Paused,
                AccessEventStatus::Ended
            ]
        );

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn snapshots_capture_who_was_inside() {
        let (repo, db_url) = setup_test_db().await;
        let event_id = running_event(&repo).await;
        record(&repo, event_id, TANELI, AccessDirection::In).await;

        let snapshot = repo
            .access_control
            .take_snapshot(event_id, Some("Äänestys 1".to_string()), Some(id(JOHANNA)))
            .await
            .unwrap();
        assert_eq!(snapshot.label.as_deref(), Some("Äänestys 1"));
        assert_eq!(snapshot.taken_by_name.as_deref(), Some("Johanna Kettunen"));
        assert_eq!(snapshot.present.len(), 1);

        // Later movements don't change a snapshot.
        record(&repo, event_id, JOHANNA, AccessDirection::In).await;
        let snapshots = repo.access_control.snapshots(event_id).await.unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].present[0].user_id, id(TANELI));

        cleanup_test_db(repo.member.pool, &db_url).await;
    }

    #[tokio::test]
    async fn deleting_an_entry_and_an_event() {
        let (repo, db_url) = setup_test_db().await;
        let event_id = running_event(&repo).await;
        record(&repo, event_id, TANELI, AccessDirection::In).await;
        let entry = repo.access_control.entries(event_id).await.unwrap()[0].id;

        // An entry of another event isn't found.
        assert!(repo
            .access_control
            .delete_entry(Uuid::new_v4(), entry)
            .await
            .is_err());
        repo.access_control
            .delete_entry(event_id, entry)
            .await
            .unwrap();
        assert!(present_ids(&repo, event_id).await.is_empty());

        assert_eq!(repo.access_control.active_events().await.unwrap().len(), 1);
        repo.access_control.delete_event(event_id).await.unwrap();
        assert!(repo.access_control.list_events().await.unwrap().is_empty());
        assert!(repo.access_control.get_event(event_id).await.is_err());

        cleanup_test_db(repo.member.pool, &db_url).await;
    }
}
