use std::collections::HashSet;
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use crate::application::ports::access_control_port::{
    AccessControlRepositoryPort, AccessPass, AccessPassError, AccessPassPort, RecordOutcome,
    StatusChangeOutcome,
};
use crate::application::ports::data_export_port::{Sheet, TabularData, WorkbookExportPort};
use crate::application::ports::member_repository_port::{
    MemberRepositoryPort, MembersWithRolesParams,
};
use crate::application::ports::repository_error::RepositoryError;
use crate::application::ports::role_repository_port::RoleRepositoryPort;
use crate::domain::{
    AccessDirection, AccessEntry, AccessEvent, AccessEventStatus, AccessMethod, AccessSnapshot,
    AccessStatusChange, NewAccessEvent, PresentMember,
};

use super::audit_log_service::AuditLogService;
use super::errors::{ServiceError, ServiceResult};

/// Whether a member's roles get them into an event right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Eligibility {
    Eligible,
    /// Holds none of the event's roles today. `ended_on` is the latest end
    /// date of an expired one, if any, so the door can say why.
    NotEligible {
        ended_on: Option<NaiveDate>,
    },
}

/// What a scan or manual entry at the door did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanOutcome {
    Recorded,
    AlreadyInside { since: DateTime<Utc> },
    NotInside,
    NotEligible { ended_on: Option<NaiveDate> },
    ExpiredPass,
    InvalidPass,
    EventNotActive,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub outcome: ScanOutcome,
    /// Unknown for an invalid or expired pass.
    pub member: Option<PresentMember>,
    pub present_count: usize,
}

#[derive(Debug, Clone)]
pub struct AccessEventDetail {
    pub event: AccessEvent,
    pub present: Vec<PresentMember>,
    pub entries: Vec<AccessEntry>,
    pub status_changes: Vec<AccessStatusChange>,
    pub snapshots: Vec<AccessSnapshot>,
}

#[derive(Debug, Clone)]
pub struct PassEvent {
    pub event: AccessEvent,
    pub eligibility: Eligibility,
}

/// What the member's home page shows: a pass while any event is running or
/// paused, and whether they get into each one.
#[derive(Debug, Clone)]
pub struct MemberAccessPass {
    pub pass: Option<AccessPass>,
    pub events: Vec<PassEvent>,
}

/// A member found by name at the door, for manual entries.
#[derive(Debug, Clone)]
pub struct DoorSearchResult {
    pub member: PresentMember,
    pub inside: bool,
    pub eligible: bool,
}

const SEARCH_LIMIT: u64 = 10;

/// Meeting access control ("kulunvalvonta"). Admins manage events; door
/// staff change their status, record movements and export the log.
#[derive(Clone)]
pub struct AccessControlService {
    repo: Arc<dyn AccessControlRepositoryPort>,
    passes: Arc<dyn AccessPassPort>,
    role_repo: Arc<dyn RoleRepositoryPort>,
    member_repo: Arc<dyn MemberRepositoryPort>,
    workbook: Arc<dyn WorkbookExportPort>,
    audit_log: AuditLogService,
}

impl AccessControlService {
    pub fn new(
        repo: Arc<dyn AccessControlRepositoryPort>,
        passes: Arc<dyn AccessPassPort>,
        role_repo: Arc<dyn RoleRepositoryPort>,
        member_repo: Arc<dyn MemberRepositoryPort>,
        workbook: Arc<dyn WorkbookExportPort>,
        audit_log: AuditLogService,
    ) -> Self {
        Self {
            repo,
            passes,
            role_repo,
            member_repo,
            workbook,
            audit_log,
        }
    }

    fn today() -> NaiveDate {
        Utc::now().date_naive()
    }

    // --- events (admin) ---

    async fn validated(&self, new: NewAccessEvent) -> ServiceResult<NewAccessEvent> {
        let name = new.name.trim().to_string();
        if name.is_empty() {
            return Err(ServiceError::Constraint(
                "event name is required".to_string(),
            ));
        }
        let mut roles: Vec<String> = Vec::new();
        for role in new.allowed_roles {
            let role = role.trim().to_string();
            if role.is_empty() || roles.contains(&role) {
                continue;
            }
            match self.role_repo.fetch_by_name(&role).await {
                Ok(_) => roles.push(role),
                Err(RepositoryError::NotFound) => {
                    return Err(ServiceError::Constraint(format!("unknown role {role:?}")));
                }
                Err(e) => return Err(e.into()),
            }
        }
        if roles.is_empty() {
            return Err(ServiceError::Constraint(
                "an event needs at least one role that gets members in".to_string(),
            ));
        }
        Ok(NewAccessEvent {
            name,
            event_date: new.event_date,
            allowed_roles: roles,
        })
    }

    pub async fn create_event(
        &self,
        new: NewAccessEvent,
        actor: Option<Uuid>,
    ) -> ServiceResult<AccessEvent> {
        let new = self.validated(new).await?;
        let event = self.repo.create_event(&new).await?;
        self.audit_log
            .log(
                actor,
                "access_event.create",
                "access_event",
                &event.id.to_string(),
                Some(serde_json::json!({ "name": event.name, "roles": event.allowed_roles })),
            )
            .await;
        Ok(event)
    }

    pub async fn update_event(
        &self,
        id: Uuid,
        new: NewAccessEvent,
        actor: Option<Uuid>,
    ) -> ServiceResult<AccessEvent> {
        let new = self.validated(new).await?;
        let event = self.repo.update_event(id, &new).await?;
        self.audit_log
            .log(
                actor,
                "access_event.update",
                "access_event",
                &id.to_string(),
                Some(serde_json::json!({ "name": event.name, "roles": event.allowed_roles })),
            )
            .await;
        Ok(event)
    }

    pub async fn delete_event(&self, id: Uuid, actor: Option<Uuid>) -> ServiceResult<()> {
        let event = self.repo.get_event(id).await?;
        self.repo.delete_event(id).await?;
        self.audit_log
            .log(
                actor,
                "access_event.delete",
                "access_event",
                &id.to_string(),
                Some(serde_json::json!({ "name": event.name })),
            )
            .await;
        Ok(())
    }

    // --- door staff ---

    pub async fn list_events(&self) -> ServiceResult<Vec<AccessEvent>> {
        Ok(self.repo.list_events().await?)
    }

    pub async fn event_detail(&self, id: Uuid) -> ServiceResult<AccessEventDetail> {
        Ok(AccessEventDetail {
            event: self.repo.get_event(id).await?,
            present: self.repo.present(id).await?,
            entries: self.repo.entries(id).await?,
            status_changes: self.repo.status_changes(id).await?,
            snapshots: self.repo.snapshots(id).await?,
        })
    }

    pub async fn change_status(
        &self,
        id: Uuid,
        next: AccessEventStatus,
        actor: Option<Uuid>,
    ) -> ServiceResult<AccessEvent> {
        match self.repo.change_status(id, next, actor).await? {
            StatusChangeOutcome::Changed => {}
            StatusChangeOutcome::NotAllowed { current } => {
                return Err(ServiceError::Constraint(format!(
                    "the event can't go from {current:?} to {next:?}"
                )));
            }
        }
        self.audit_log
            .log(
                actor,
                "access_event.status",
                "access_event",
                &id.to_string(),
                Some(serde_json::json!({ "status": format!("{next:?}") })),
            )
            .await;
        Ok(self.repo.get_event(id).await?)
    }

    /// Whether `user_id` holds one of the event's roles today.
    async fn eligibility(&self, event: &AccessEvent, user_id: Uuid) -> ServiceResult<Eligibility> {
        let today = Self::today();
        let memberships = self.role_repo.fetch_roles_by_member(&user_id).await?;
        let relevant = memberships
            .iter()
            .filter(|m| event.allowed_roles.contains(&m.role_name.0));
        let mut ended_on: Option<NaiveDate> = None;
        for m in relevant {
            if m.is_active_on(today) {
                return Ok(Eligibility::Eligible);
            }
            if let Some(until) = m.valid_until.filter(|until| *until < today) {
                ended_on = ended_on.max(Some(until));
            }
        }
        Ok(Eligibility::NotEligible { ended_on })
    }

    async fn present_member(&self, user_id: Uuid) -> ServiceResult<PresentMember> {
        let person = self.member_repo.fetch_one(user_id).await?;
        Ok(PresentMember {
            user_id,
            name: format!("{} {}", person.first_name, person.last_name),
            email: person.email.as_str().to_string(),
        })
    }

    /// Records `user_id` going `direction`. Going in needs one of the
    /// event's roles; going out never does, so nobody is stuck inside.
    async fn record(
        &self,
        event_id: Uuid,
        user_id: Uuid,
        direction: AccessDirection,
        method: AccessMethod,
        actor: Option<Uuid>,
    ) -> ServiceResult<ScanResult> {
        let event = self.repo.get_event(event_id).await?;
        let member = self.present_member(user_id).await?;
        let eligibility = match direction {
            AccessDirection::In if event.status.accepts_entries() => {
                self.eligibility(&event, user_id).await?
            }
            _ => Eligibility::Eligible,
        };
        let outcome = if !event.status.accepts_entries() {
            ScanOutcome::EventNotActive
        } else if let Eligibility::NotEligible { ended_on } = eligibility {
            ScanOutcome::NotEligible { ended_on }
        } else {
            match self
                .repo
                .record_entry(event_id, user_id, direction, method, actor)
                .await?
            {
                RecordOutcome::Recorded => ScanOutcome::Recorded,
                RecordOutcome::AlreadyInside { since } => ScanOutcome::AlreadyInside { since },
                RecordOutcome::NotInside => ScanOutcome::NotInside,
                RecordOutcome::EventNotActive => ScanOutcome::EventNotActive,
            }
        };
        Ok(ScanResult {
            outcome,
            member: Some(member),
            present_count: self.repo.present(event_id).await?.len(),
        })
    }

    pub async fn scan(
        &self,
        event_id: Uuid,
        token: &str,
        direction: AccessDirection,
        actor: Option<Uuid>,
    ) -> ServiceResult<ScanResult> {
        let user_id = match self.passes.verify(token) {
            Ok(user_id) => user_id,
            Err(e) => {
                return Ok(ScanResult {
                    outcome: match e {
                        AccessPassError::Expired => ScanOutcome::ExpiredPass,
                        AccessPassError::Invalid => ScanOutcome::InvalidPass,
                    },
                    member: None,
                    present_count: self.repo.present(event_id).await?.len(),
                });
            }
        };
        self.record(event_id, user_id, direction, AccessMethod::Qr, actor)
            .await
    }

    pub async fn record_manual(
        &self,
        event_id: Uuid,
        user_id: Uuid,
        direction: AccessDirection,
        actor: Option<Uuid>,
    ) -> ServiceResult<ScanResult> {
        self.record(event_id, user_id, direction, AccessMethod::Manual, actor)
            .await
    }

    /// Members matching `query` by name or email, with whether each is inside
    /// and holds one of the event's roles today.
    pub async fn search_members(
        &self,
        event_id: Uuid,
        query: &str,
    ) -> ServiceResult<Vec<DoorSearchResult>> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let event = self.repo.get_event(event_id).await?;
        let inside: HashSet<Uuid> = self
            .repo
            .present(event_id)
            .await?
            .into_iter()
            .map(|m| m.user_id)
            .collect();
        let today = Self::today();
        // With both dates set to today, role_names lists only roles held today.
        let members = self
            .member_repo
            .fetch_members_with_roles(MembersWithRolesParams {
                search: Some(query.to_string()),
                page_size: Some(SEARCH_LIMIT),
                valid_from: Some(today),
                valid_until: Some(today),
                ..Default::default()
            })
            .await?;
        Ok(members
            .into_iter()
            .map(|m| {
                let user_id = m.person.id.0;
                DoorSearchResult {
                    eligible: m.role_names.iter().any(|r| event.allowed_roles.contains(r)),
                    inside: inside.contains(&user_id),
                    member: PresentMember {
                        user_id,
                        name: format!("{} {}", m.person.first_name, m.person.last_name),
                        email: m.person.email.as_str().to_string(),
                    },
                }
            })
            .collect())
    }

    pub async fn delete_entry(
        &self,
        event_id: Uuid,
        entry_id: Uuid,
        actor: Option<Uuid>,
    ) -> ServiceResult<()> {
        self.repo.delete_entry(event_id, entry_id).await?;
        self.audit_log
            .log(
                actor,
                "access_entry.delete",
                "access_event",
                &event_id.to_string(),
                Some(serde_json::json!({ "entry_id": entry_id })),
            )
            .await;
        Ok(())
    }

    pub async fn take_snapshot(
        &self,
        event_id: Uuid,
        label: Option<String>,
        actor: Option<Uuid>,
    ) -> ServiceResult<AccessSnapshot> {
        self.repo.get_event(event_id).await?;
        let label = label
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty());
        Ok(self.repo.take_snapshot(event_id, label, actor).await?)
    }

    // --- members ---

    pub async fn member_pass(&self, user_id: Uuid) -> ServiceResult<MemberAccessPass> {
        let active = self.repo.active_events().await?;
        let mut events = Vec::with_capacity(active.len());
        for event in active {
            let eligibility = self.eligibility(&event, user_id).await?;
            events.push(PassEvent { event, eligibility });
        }
        Ok(MemberAccessPass {
            pass: (!events.is_empty()).then(|| self.passes.issue(user_id)),
            events,
        })
    }

    // --- export ---

    /// The event as a workbook: per-member summary, the full log including
    /// status changes, and the snapshots.
    pub async fn export(&self, event_id: Uuid) -> ServiceResult<(AccessEvent, Vec<u8>)> {
        let detail = self.event_detail(event_id).await?;
        let sheets = export_sheets(&detail);
        let bytes = self.workbook.write(&sheets)?;
        Ok((detail.event, bytes))
    }

    pub fn export_content_type(&self) -> &str {
        self.workbook.content_type()
    }

    pub fn export_file_extension(&self) -> &str {
        self.workbook.file_extension()
    }
}

/// Times in the timezone the meetings are held in.
fn local_time(at: DateTime<Utc>) -> String {
    at.with_timezone(&chrono_tz::Europe::Helsinki)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

fn direction_label(direction: AccessDirection) -> &'static str {
    match direction {
        AccessDirection::In => "In",
        AccessDirection::Out => "Out",
    }
}

fn method_label(method: AccessMethod) -> &'static str {
    match method {
        AccessMethod::Qr => "QR",
        AccessMethod::Manual => "Manual",
        AccessMethod::Auto => "Automatic (event ended)",
    }
}

fn status_label(status: AccessEventStatus) -> &'static str {
    match status {
        AccessEventStatus::NotStarted => "Not started",
        AccessEventStatus::Running => "Event running",
        AccessEventStatus::Paused => "Event paused",
        AccessEventStatus::Ended => "Event ended",
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|v| v.to_string()).collect()
}

fn export_sheets(detail: &AccessEventDetail) -> Vec<Sheet> {
    // Summary: one row per member, in order of first arrival.
    let mut summary: Vec<(Uuid, Vec<String>)> = Vec::new();
    for entry in &detail.entries {
        let index = match summary.iter().position(|(id, _)| *id == entry.user_id) {
            Some(index) => index,
            None => {
                summary.push((
                    entry.user_id,
                    vec![
                        entry.member_name.clone(),
                        entry.member_email.clone(),
                        String::new(),
                        String::new(),
                    ],
                ));
                summary.len() - 1
            }
        };
        let row = &mut summary[index].1;
        match entry.direction {
            AccessDirection::In if row[2].is_empty() => row[2] = local_time(entry.recorded_at),
            AccessDirection::Out => row[3] = local_time(entry.recorded_at),
            AccessDirection::In => {}
        }
    }
    let present: HashSet<Uuid> = detail.present.iter().map(|m| m.user_id).collect();
    let summary_rows = summary
        .into_iter()
        .map(|(id, mut row)| {
            // Someone back inside after leaving has no final departure.
            if present.contains(&id) {
                row[3] = String::new();
            }
            row
        })
        .collect();

    // Log: entries and status changes, in time order.
    let mut log: Vec<(DateTime<Utc>, Vec<String>)> = detail
        .entries
        .iter()
        .map(|e| {
            (
                e.recorded_at,
                vec![
                    local_time(e.recorded_at),
                    e.member_name.clone(),
                    e.member_email.clone(),
                    direction_label(e.direction).to_string(),
                    method_label(e.method).to_string(),
                    e.recorded_by_name.clone().unwrap_or_default(),
                ],
            )
        })
        .chain(detail.status_changes.iter().map(|c| {
            (
                c.changed_at,
                vec![
                    local_time(c.changed_at),
                    String::new(),
                    String::new(),
                    status_label(c.status).to_string(),
                    String::new(),
                    c.changed_by_name.clone().unwrap_or_default(),
                ],
            )
        }))
        .collect();
    log.sort_by_key(|(at, _)| *at);

    // Snapshots: one row per member present at each snapshot.
    let mut snapshot_rows = Vec::new();
    for s in &detail.snapshots {
        let label = s.label.clone().unwrap_or_default();
        if s.present.is_empty() {
            snapshot_rows.push(vec![
                label.clone(),
                local_time(s.taken_at),
                "0".to_string(),
                String::new(),
                String::new(),
            ]);
        }
        for m in &s.present {
            snapshot_rows.push(vec![
                label.clone(),
                local_time(s.taken_at),
                s.present.len().to_string(),
                m.name.clone(),
                m.email.clone(),
            ]);
        }
    }

    vec![
        Sheet {
            name: "Summary".to_string(),
            data: TabularData {
                headers: strings(&["Name", "Email", "First in", "Last out"]),
                rows: summary_rows,
            },
        },
        Sheet {
            name: "Log".to_string(),
            data: TabularData {
                headers: strings(&["Time", "Name", "Email", "Event", "Method", "Recorded by"]),
                rows: log.into_iter().map(|(_, row)| row).collect(),
            },
        },
        Sheet {
            name: "Snapshots".to_string(),
            data: TabularData {
                headers: strings(&["Snapshot", "Time", "Present", "Name", "Email"]),
                rows: snapshot_rows,
            },
        },
    ]
}
