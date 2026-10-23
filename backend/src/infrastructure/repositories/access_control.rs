use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::application::ports::access_control_port::{
    AccessControlRepositoryPort, RecordOutcome, StatusChangeOutcome,
};
use crate::application::ports::repository_error::RepositoryError;
use crate::domain::{
    AccessDirection, AccessEntry, AccessEvent, AccessEventStatus, AccessMethod, AccessSnapshot,
    AccessStatusChange, NewAccessEvent, PresentMember,
};

// ---------------------------------------------------------------------------
// DAOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "access_event_status", rename_all = "snake_case")]
enum StatusDAO {
    NotStarted,
    Running,
    Paused,
    Ended,
}

impl From<StatusDAO> for AccessEventStatus {
    fn from(s: StatusDAO) -> Self {
        match s {
            StatusDAO::NotStarted => Self::NotStarted,
            StatusDAO::Running => Self::Running,
            StatusDAO::Paused => Self::Paused,
            StatusDAO::Ended => Self::Ended,
        }
    }
}

impl From<AccessEventStatus> for StatusDAO {
    fn from(s: AccessEventStatus) -> Self {
        match s {
            AccessEventStatus::NotStarted => Self::NotStarted,
            AccessEventStatus::Running => Self::Running,
            AccessEventStatus::Paused => Self::Paused,
            AccessEventStatus::Ended => Self::Ended,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "access_entry_direction", rename_all = "lowercase")]
enum DirectionDAO {
    In,
    Out,
}

impl From<DirectionDAO> for AccessDirection {
    fn from(d: DirectionDAO) -> Self {
        match d {
            DirectionDAO::In => Self::In,
            DirectionDAO::Out => Self::Out,
        }
    }
}

impl From<AccessDirection> for DirectionDAO {
    fn from(d: AccessDirection) -> Self {
        match d {
            AccessDirection::In => Self::In,
            AccessDirection::Out => Self::Out,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "access_entry_method", rename_all = "lowercase")]
enum MethodDAO {
    Qr,
    Manual,
    Auto,
}

impl From<MethodDAO> for AccessMethod {
    fn from(m: MethodDAO) -> Self {
        match m {
            MethodDAO::Qr => Self::Qr,
            MethodDAO::Manual => Self::Manual,
            MethodDAO::Auto => Self::Auto,
        }
    }
}

impl From<AccessMethod> for MethodDAO {
    fn from(m: AccessMethod) -> Self {
        match m {
            AccessMethod::Qr => Self::Qr,
            AccessMethod::Manual => Self::Manual,
            AccessMethod::Auto => Self::Auto,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct EventDAO {
    id: Uuid,
    name: String,
    event_date: NaiveDate,
    allowed_roles: Vec<String>,
    status: StatusDAO,
    created_at: DateTime<Utc>,
}

impl From<EventDAO> for AccessEvent {
    fn from(e: EventDAO) -> Self {
        Self {
            id: e.id,
            name: e.name,
            event_date: e.event_date,
            allowed_roles: e.allowed_roles,
            status: e.status.into(),
            created_at: e.created_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct EntryDAO {
    id: Uuid,
    user_id: Uuid,
    member_name: String,
    member_email: String,
    direction: DirectionDAO,
    method: MethodDAO,
    recorded_at: DateTime<Utc>,
    recorded_by_name: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct StatusChangeDAO {
    status: StatusDAO,
    changed_at: DateTime<Utc>,
    changed_by_name: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct PresentDAO {
    user_id: Uuid,
    name: String,
    email: String,
}

impl From<PresentDAO> for PresentMember {
    fn from(p: PresentDAO) -> Self {
        Self {
            user_id: p.user_id,
            name: p.name,
            email: p.email,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct SnapshotDAO {
    id: Uuid,
    label: Option<String>,
    taken_at: DateTime<Utc>,
    taken_by_name: Option<String>,
    user_ids: Vec<Uuid>,
}

/// Members whose latest entry in event `$1` is `in`.
const PRESENT_SQL: &str = r#"
    SELECT m.user_id, m.full_name AS name, m.email
    FROM (
        SELECT DISTINCT ON (user_id) user_id, direction
        FROM AccessEntry
        WHERE event_id = $1
        ORDER BY user_id, recorded_at DESC, id DESC
    ) latest
    JOIN Member m ON m.user_id = latest.user_id
    WHERE latest.direction = 'in'
    ORDER BY m.full_name
"#;

// ---------------------------------------------------------------------------
// Repository
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AccessControlRepo {
    pub pool: PgPool,
}

impl AccessControlRepo {
    /// Locks the event row for the rest of `tx`, serializing every write to
    /// one event so the status and presence checks can't race.
    async fn lock_event(
        tx: &mut Transaction<'_, Postgres>,
        id: Uuid,
    ) -> Result<AccessEventStatus, RepositoryError> {
        let status: StatusDAO =
            sqlx::query_scalar("SELECT status FROM AccessEvent WHERE id = $1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut **tx)
                .await?
                .ok_or(RepositoryError::NotFound)?;
        Ok(status.into())
    }

    /// When the member last went in, if they are inside now.
    async fn inside_since(
        tx: &mut Transaction<'_, Postgres>,
        event_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<DateTime<Utc>>, RepositoryError> {
        let latest: Option<(DirectionDAO, DateTime<Utc>)> = sqlx::query_as(
            "SELECT direction, recorded_at FROM AccessEntry
             WHERE event_id = $1 AND user_id = $2
             ORDER BY recorded_at DESC, id DESC LIMIT 1",
        )
        .bind(event_id)
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await?;
        Ok(latest.and_then(|(direction, at)| (direction == DirectionDAO::In).then_some(at)))
    }

    async fn present_with<'e, E>(
        executor: E,
        event_id: Uuid,
    ) -> Result<Vec<PresentMember>, RepositoryError>
    where
        E: sqlx::Executor<'e, Database = Postgres>,
    {
        let rows: Vec<PresentDAO> = sqlx::query_as(PRESENT_SQL)
            .bind(event_id)
            .fetch_all(executor)
            .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn members_by_id(&self, ids: &[Uuid]) -> Result<Vec<PresentMember>, RepositoryError> {
        let rows: Vec<PresentDAO> = sqlx::query_as(
            "SELECT user_id, full_name AS name, email FROM Member
             WHERE user_id = ANY($1) ORDER BY full_name",
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn snapshot_from(&self, s: SnapshotDAO) -> Result<AccessSnapshot, RepositoryError> {
        Ok(AccessSnapshot {
            id: s.id,
            label: s.label,
            taken_at: s.taken_at,
            taken_by_name: s.taken_by_name,
            present: self.members_by_id(&s.user_ids).await?,
        })
    }
}

#[async_trait::async_trait]
impl AccessControlRepositoryPort for AccessControlRepo {
    async fn create_event(&self, new: &NewAccessEvent) -> Result<AccessEvent, RepositoryError> {
        let row: EventDAO = sqlx::query_as(
            "INSERT INTO AccessEvent (name, event_date, allowed_roles)
             VALUES ($1, $2, $3) RETURNING id, name, event_date, allowed_roles, status, created_at",
        )
        .bind(&new.name)
        .bind(new.event_date)
        .bind(&new.allowed_roles)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn update_event(
        &self,
        id: Uuid,
        new: &NewAccessEvent,
    ) -> Result<AccessEvent, RepositoryError> {
        let row: EventDAO = sqlx::query_as(
            "UPDATE AccessEvent SET name = $2, event_date = $3, allowed_roles = $4
             WHERE id = $1 RETURNING id, name, event_date, allowed_roles, status, created_at",
        )
        .bind(id)
        .bind(&new.name)
        .bind(new.event_date)
        .bind(&new.allowed_roles)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(RepositoryError::NotFound)?;
        Ok(row.into())
    }

    async fn delete_event(&self, id: Uuid) -> Result<(), RepositoryError> {
        let result = sqlx::query("DELETE FROM AccessEvent WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }

    async fn list_events(&self) -> Result<Vec<AccessEvent>, RepositoryError> {
        let rows: Vec<EventDAO> = sqlx::query_as(
            "SELECT id, name, event_date, allowed_roles, status, created_at FROM AccessEvent ORDER BY event_date DESC, created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get_event(&self, id: Uuid) -> Result<AccessEvent, RepositoryError> {
        let row: EventDAO = sqlx::query_as(
            "SELECT id, name, event_date, allowed_roles, status, created_at FROM AccessEvent WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(RepositoryError::NotFound)?;
        Ok(row.into())
    }

    async fn active_events(&self) -> Result<Vec<AccessEvent>, RepositoryError> {
        let rows: Vec<EventDAO> = sqlx::query_as(
            "SELECT id, name, event_date, allowed_roles, status, created_at FROM AccessEvent
             WHERE status IN ('running', 'paused') ORDER BY event_date, created_at",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn change_status(
        &self,
        id: Uuid,
        next: AccessEventStatus,
        actor: Option<Uuid>,
    ) -> Result<StatusChangeOutcome, RepositoryError> {
        let mut tx = self.pool.begin().await?;
        let current = Self::lock_event(&mut tx, id).await?;
        if !current.can_change_to(next) {
            return Ok(StatusChangeOutcome::NotAllowed { current });
        }
        if next == AccessEventStatus::Ended {
            // Everyone still inside leaves when the event closes.
            let inside = Self::present_with(&mut *tx, id).await?;
            for member in inside {
                sqlx::query(
                    "INSERT INTO AccessEntry (event_id, user_id, direction, method)
                     VALUES ($1, $2, 'out', 'auto')",
                )
                .bind(id)
                .bind(member.user_id)
                .execute(&mut *tx)
                .await?;
            }
        }
        sqlx::query("UPDATE AccessEvent SET status = $2 WHERE id = $1")
            .bind(id)
            .bind(StatusDAO::from(next))
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO AccessStatusChange (event_id, status, changed_by) VALUES ($1, $2, $3)",
        )
        .bind(id)
        .bind(StatusDAO::from(next))
        .bind(actor)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(StatusChangeOutcome::Changed)
    }

    async fn record_entry(
        &self,
        event_id: Uuid,
        user_id: Uuid,
        direction: AccessDirection,
        method: AccessMethod,
        actor: Option<Uuid>,
    ) -> Result<RecordOutcome, RepositoryError> {
        let mut tx = self.pool.begin().await?;
        if !Self::lock_event(&mut tx, event_id).await?.accepts_entries() {
            return Ok(RecordOutcome::EventNotActive);
        }
        let since = Self::inside_since(&mut tx, event_id, user_id).await?;
        match (direction, since) {
            (AccessDirection::In, Some(since)) => {
                return Ok(RecordOutcome::AlreadyInside { since });
            }
            (AccessDirection::Out, None) => return Ok(RecordOutcome::NotInside),
            _ => {}
        }
        sqlx::query(
            "INSERT INTO AccessEntry (event_id, user_id, direction, method, recorded_by)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(event_id)
        .bind(user_id)
        .bind(DirectionDAO::from(direction))
        .bind(MethodDAO::from(method))
        .bind(actor)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(RecordOutcome::Recorded)
    }

    async fn delete_entry(&self, event_id: Uuid, entry_id: Uuid) -> Result<(), RepositoryError> {
        let result = sqlx::query("DELETE FROM AccessEntry WHERE id = $1 AND event_id = $2")
            .bind(entry_id)
            .bind(event_id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }

    async fn entries(&self, event_id: Uuid) -> Result<Vec<AccessEntry>, RepositoryError> {
        let rows: Vec<EntryDAO> = sqlx::query_as(
            "SELECT e.id, e.user_id, m.full_name AS member_name, m.email AS member_email,
                    e.direction, e.method, e.recorded_at, r.full_name AS recorded_by_name
             FROM AccessEntry e
             JOIN Member m ON m.user_id = e.user_id
             LEFT JOIN Member r ON r.user_id = e.recorded_by
             WHERE e.event_id = $1
             ORDER BY e.recorded_at, e.id",
        )
        .bind(event_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|e| AccessEntry {
                id: e.id,
                user_id: e.user_id,
                member_name: e.member_name,
                member_email: e.member_email,
                direction: e.direction.into(),
                method: e.method.into(),
                recorded_at: e.recorded_at,
                recorded_by_name: e.recorded_by_name,
            })
            .collect())
    }

    async fn present(&self, event_id: Uuid) -> Result<Vec<PresentMember>, RepositoryError> {
        Self::present_with(&self.pool, event_id).await
    }

    async fn status_changes(
        &self,
        event_id: Uuid,
    ) -> Result<Vec<AccessStatusChange>, RepositoryError> {
        let rows: Vec<StatusChangeDAO> = sqlx::query_as(
            "SELECT c.status, c.changed_at, m.full_name AS changed_by_name
             FROM AccessStatusChange c
             LEFT JOIN Member m ON m.user_id = c.changed_by
             WHERE c.event_id = $1
             ORDER BY c.changed_at, c.id",
        )
        .bind(event_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|c| AccessStatusChange {
                status: c.status.into(),
                changed_at: c.changed_at,
                changed_by_name: c.changed_by_name,
            })
            .collect())
    }

    async fn take_snapshot(
        &self,
        event_id: Uuid,
        label: Option<String>,
        actor: Option<Uuid>,
    ) -> Result<AccessSnapshot, RepositoryError> {
        let mut tx = self.pool.begin().await?;
        Self::lock_event(&mut tx, event_id).await?;
        let present = Self::present_with(&mut *tx, event_id).await?;
        let ids: Vec<Uuid> = present.iter().map(|m| m.user_id).collect();
        let row: SnapshotDAO = sqlx::query_as(
            "WITH inserted AS (
                 INSERT INTO AccessSnapshot (event_id, label, taken_by, user_ids)
                 VALUES ($1, $2, $3, $4)
                 RETURNING id, label, taken_at, taken_by, user_ids
             )
             SELECT i.id, i.label, i.taken_at, m.full_name AS taken_by_name, i.user_ids
             FROM inserted i LEFT JOIN Member m ON m.user_id = i.taken_by",
        )
        .bind(event_id)
        .bind(label)
        .bind(actor)
        .bind(&ids)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(AccessSnapshot {
            id: row.id,
            label: row.label,
            taken_at: row.taken_at,
            taken_by_name: row.taken_by_name,
            present,
        })
    }

    async fn snapshots(&self, event_id: Uuid) -> Result<Vec<AccessSnapshot>, RepositoryError> {
        let rows: Vec<SnapshotDAO> = sqlx::query_as(
            "SELECT s.id, s.label, s.taken_at, m.full_name AS taken_by_name, s.user_ids
             FROM AccessSnapshot s
             LEFT JOIN Member m ON m.user_id = s.taken_by
             WHERE s.event_id = $1
             ORDER BY s.taken_at, s.id",
        )
        .bind(event_id)
        .fetch_all(&self.pool)
        .await?;
        let mut snapshots = Vec::with_capacity(rows.len());
        for row in rows {
            snapshots.push(self.snapshot_from(row).await?);
        }
        Ok(snapshots)
    }
}
