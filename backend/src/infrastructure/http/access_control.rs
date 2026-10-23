//! Meeting access control ("kulunvalvonta"). Door staff and admins use
//! `/api/access-control`; members fetch their rotating pass from
//! `/api/access-pass`.

use axum::{
    body::Body,
    debug_handler,
    extract::{Path, Query, State},
    http::{Response, StatusCode},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    application::services::{
        access_control_service::{
            AccessEventDetail, DoorSearchResult, Eligibility, MemberAccessPass, ScanOutcome,
            ScanResult,
        },
        authentication_service::AuthenticatedUser,
    },
    domain::{
        AccessDirection, AccessEntry, AccessEvent, AccessEventStatus, AccessMethod, AccessSnapshot,
        AccessStatusChange, NewAccessEvent, PresentMember,
    },
};

use super::{
    errors::{ApiError, ApiResult},
    middleware::{check_access_control_permission, check_auth},
    AppState,
};

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename = "AccessEventStatus")]
pub enum AccessEventStatusDTO {
    NotStarted,
    Running,
    Paused,
    Ended,
}

impl From<AccessEventStatus> for AccessEventStatusDTO {
    fn from(s: AccessEventStatus) -> Self {
        match s {
            AccessEventStatus::NotStarted => Self::NotStarted,
            AccessEventStatus::Running => Self::Running,
            AccessEventStatus::Paused => Self::Paused,
            AccessEventStatus::Ended => Self::Ended,
        }
    }
}

impl From<AccessEventStatusDTO> for AccessEventStatus {
    fn from(s: AccessEventStatusDTO) -> Self {
        match s {
            AccessEventStatusDTO::NotStarted => Self::NotStarted,
            AccessEventStatusDTO::Running => Self::Running,
            AccessEventStatusDTO::Paused => Self::Paused,
            AccessEventStatusDTO::Ended => Self::Ended,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename = "AccessDirection")]
pub enum AccessDirectionDTO {
    In,
    Out,
}

impl From<AccessDirection> for AccessDirectionDTO {
    fn from(d: AccessDirection) -> Self {
        match d {
            AccessDirection::In => Self::In,
            AccessDirection::Out => Self::Out,
        }
    }
}

impl From<AccessDirectionDTO> for AccessDirection {
    fn from(d: AccessDirectionDTO) -> Self {
        match d {
            AccessDirectionDTO::In => Self::In,
            AccessDirectionDTO::Out => Self::Out,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename = "AccessMethod")]
pub enum AccessMethodDTO {
    Qr,
    Manual,
    Auto,
}

impl From<AccessMethod> for AccessMethodDTO {
    fn from(m: AccessMethod) -> Self {
        match m {
            AccessMethod::Qr => Self::Qr,
            AccessMethod::Manual => Self::Manual,
            AccessMethod::Auto => Self::Auto,
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[ts(export, rename = "AccessEvent")]
pub struct AccessEventDTO {
    id: Uuid,
    name: String,
    event_date: NaiveDate,
    allowed_roles: Vec<String>,
    status: AccessEventStatusDTO,
    created_at: DateTime<Utc>,
}

impl From<AccessEvent> for AccessEventDTO {
    fn from(e: AccessEvent) -> Self {
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

#[derive(Debug, Deserialize, TS)]
#[ts(export, rename = "SaveAccessEvent")]
struct SaveAccessEventDTO {
    name: String,
    event_date: NaiveDate,
    allowed_roles: Vec<String>,
}

impl From<SaveAccessEventDTO> for NewAccessEvent {
    fn from(d: SaveAccessEventDTO) -> Self {
        Self {
            name: d.name,
            event_date: d.event_date,
            allowed_roles: d.allowed_roles,
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[ts(export, rename = "AccessPresentMember")]
pub struct PresentMemberDTO {
    user_id: Uuid,
    name: String,
    email: String,
}

impl From<PresentMember> for PresentMemberDTO {
    fn from(m: PresentMember) -> Self {
        Self {
            user_id: m.user_id,
            name: m.name,
            email: m.email,
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[ts(export, rename = "AccessEntry")]
pub struct AccessEntryDTO {
    id: Uuid,
    user_id: Uuid,
    member_name: String,
    member_email: String,
    direction: AccessDirectionDTO,
    method: AccessMethodDTO,
    recorded_at: DateTime<Utc>,
    recorded_by_name: Option<String>,
}

impl From<AccessEntry> for AccessEntryDTO {
    fn from(e: AccessEntry) -> Self {
        Self {
            id: e.id,
            user_id: e.user_id,
            member_name: e.member_name,
            member_email: e.member_email,
            direction: e.direction.into(),
            method: e.method.into(),
            recorded_at: e.recorded_at,
            recorded_by_name: e.recorded_by_name,
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[ts(export, rename = "AccessStatusChange")]
pub struct AccessStatusChangeDTO {
    status: AccessEventStatusDTO,
    changed_at: DateTime<Utc>,
    changed_by_name: Option<String>,
}

impl From<AccessStatusChange> for AccessStatusChangeDTO {
    fn from(c: AccessStatusChange) -> Self {
        Self {
            status: c.status.into(),
            changed_at: c.changed_at,
            changed_by_name: c.changed_by_name,
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[ts(export, rename = "AccessSnapshot")]
pub struct AccessSnapshotDTO {
    id: Uuid,
    label: Option<String>,
    taken_at: DateTime<Utc>,
    taken_by_name: Option<String>,
    present: Vec<PresentMemberDTO>,
}

impl From<AccessSnapshot> for AccessSnapshotDTO {
    fn from(s: AccessSnapshot) -> Self {
        Self {
            id: s.id,
            label: s.label,
            taken_at: s.taken_at,
            taken_by_name: s.taken_by_name,
            present: s.present.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[ts(export, rename = "AccessEventDetail")]
pub struct AccessEventDetailDTO {
    event: AccessEventDTO,
    present: Vec<PresentMemberDTO>,
    entries: Vec<AccessEntryDTO>,
    status_changes: Vec<AccessStatusChangeDTO>,
    snapshots: Vec<AccessSnapshotDTO>,
}

impl From<AccessEventDetail> for AccessEventDetailDTO {
    fn from(d: AccessEventDetail) -> Self {
        Self {
            event: d.event.into(),
            present: d.present.into_iter().map(Into::into).collect(),
            entries: d.entries.into_iter().map(Into::into).collect(),
            status_changes: d.status_changes.into_iter().map(Into::into).collect(),
            snapshots: d.snapshots.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export, rename = "ScanOutcome")]
pub enum ScanOutcomeDTO {
    Recorded,
    AlreadyInside { since: DateTime<Utc> },
    NotInside,
    NotEligible { ended_on: Option<NaiveDate> },
    ExpiredPass,
    InvalidPass,
    EventNotActive,
}

impl From<ScanOutcome> for ScanOutcomeDTO {
    fn from(o: ScanOutcome) -> Self {
        match o {
            ScanOutcome::Recorded => Self::Recorded,
            ScanOutcome::AlreadyInside { since } => Self::AlreadyInside { since },
            ScanOutcome::NotInside => Self::NotInside,
            ScanOutcome::NotEligible { ended_on } => Self::NotEligible { ended_on },
            ScanOutcome::ExpiredPass => Self::ExpiredPass,
            ScanOutcome::InvalidPass => Self::InvalidPass,
            ScanOutcome::EventNotActive => Self::EventNotActive,
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[ts(export, rename = "ScanResult")]
pub struct ScanResultDTO {
    outcome: ScanOutcomeDTO,
    member: Option<PresentMemberDTO>,
    present_count: usize,
}

impl From<ScanResult> for ScanResultDTO {
    fn from(r: ScanResult) -> Self {
        Self {
            outcome: r.outcome.into(),
            member: r.member.map(Into::into),
            present_count: r.present_count,
        }
    }
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, rename = "ScanRequest")]
struct ScanRequestDTO {
    token: String,
    direction: AccessDirectionDTO,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, rename = "ManualEntryRequest")]
struct ManualEntryRequestDTO {
    user_id: Uuid,
    direction: AccessDirectionDTO,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, rename = "ChangeAccessEventStatus")]
struct ChangeStatusDTO {
    status: AccessEventStatusDTO,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, rename = "TakeAccessSnapshot")]
struct TakeSnapshotDTO {
    label: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, rename = "DoorSearchResult")]
pub struct DoorSearchResultDTO {
    member: PresentMemberDTO,
    inside: bool,
    eligible: bool,
}

impl From<DoorSearchResult> for DoorSearchResultDTO {
    fn from(r: DoorSearchResult) -> Self {
        Self {
            member: r.member.into(),
            inside: r.inside,
            eligible: r.eligible,
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[ts(export, rename = "PassEvent")]
pub struct PassEventDTO {
    id: Uuid,
    name: String,
    status: AccessEventStatusDTO,
    eligible: bool,
    /// For an ineligible member whose role expired: when it ended.
    ended_on: Option<NaiveDate>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, rename = "MemberAccessPass")]
pub struct MemberAccessPassDTO {
    /// Present only while some event is running or paused.
    token: Option<String>,
    expires_at: Option<DateTime<Utc>>,
    events: Vec<PassEventDTO>,
    /// Whether the member may open the door-staff pages.
    can_control: bool,
}

impl MemberAccessPassDTO {
    fn new(pass: MemberAccessPass, can_control: bool) -> Self {
        Self {
            token: pass.pass.as_ref().map(|p| p.token.clone()),
            expires_at: pass.pass.as_ref().map(|p| p.expires_at),
            events: pass
                .events
                .into_iter()
                .map(|e| {
                    let (eligible, ended_on) = match e.eligibility {
                        Eligibility::Eligible => (true, None),
                        Eligibility::NotEligible { ended_on } => (false, ended_on),
                    };
                    PassEventDTO {
                        id: e.event.id,
                        name: e.event.name,
                        status: e.event.status.into(),
                        eligible,
                        ended_on,
                    }
                })
                .collect(),
            can_control,
        }
    }
}

// ---------------------------------------------------------------------------
// Routers
// ---------------------------------------------------------------------------

/// Door staff and admins. Creating, editing and deleting events is further
/// limited to admins in the handlers.
pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/events", get(list_events).post(create_event))
        .route(
            "/events/{id}",
            get(get_event).put(update_event).delete(delete_event),
        )
        .route("/events/{id}/status", post(change_status))
        .route("/events/{id}/scan", post(scan))
        .route("/events/{id}/manual", post(manual_entry))
        .route("/events/{id}/search", get(search_members))
        .route("/events/{id}/entries/{entry_id}", delete(delete_entry))
        .route("/events/{id}/snapshots", post(take_snapshot))
        .route("/events/{id}/export", get(export_event))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            check_access_control_permission,
        ))
        .layer(axum::middleware::from_fn_with_state(state, check_auth))
}

/// Any signed-in member; nested under the already-authenticated router.
pub fn pass_router() -> Router<AppState> {
    Router::new().route("/", get(get_member_pass))
}

fn actor(user: &Option<AuthenticatedUser>) -> Option<Uuid> {
    user.as_ref().map(|u| u.user_id)
}

fn require_admin(state: &AppState, user: &Option<AuthenticatedUser>) -> Result<(), ApiError> {
    match user {
        Some(u) if state.authentication_service.is_admin(u) => Ok(()),
        _ => Err(ApiError::Forbidden),
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[debug_handler]
async fn list_events(State(state): State<AppState>) -> ApiResult<Json<Vec<AccessEventDTO>>> {
    let events = state.access_control_service.list_events().await?;
    Ok(Json(events.into_iter().map(Into::into).collect()))
}

#[debug_handler]
async fn create_event(
    Extension(user): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Json(body): Json<SaveAccessEventDTO>,
) -> ApiResult<Json<AccessEventDTO>> {
    require_admin(&state, &user)?;
    let event = state
        .access_control_service
        .create_event(body.into(), actor(&user))
        .await?;
    Ok(Json(event.into()))
}

#[debug_handler]
async fn get_event(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<AccessEventDetailDTO>> {
    let detail = state.access_control_service.event_detail(id).await?;
    Ok(Json(detail.into()))
}

#[debug_handler]
async fn update_event(
    Extension(user): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<SaveAccessEventDTO>,
) -> ApiResult<Json<AccessEventDTO>> {
    require_admin(&state, &user)?;
    let event = state
        .access_control_service
        .update_event(id, body.into(), actor(&user))
        .await?;
    Ok(Json(event.into()))
}

#[debug_handler]
async fn delete_event(
    Extension(user): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<()> {
    require_admin(&state, &user)?;
    state
        .access_control_service
        .delete_event(id, actor(&user))
        .await?;
    Ok(())
}

#[debug_handler]
async fn change_status(
    Extension(user): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<ChangeStatusDTO>,
) -> ApiResult<Json<AccessEventDTO>> {
    let event = state
        .access_control_service
        .change_status(id, body.status.into(), actor(&user))
        .await?;
    Ok(Json(event.into()))
}

#[debug_handler]
async fn scan(
    Extension(user): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<ScanRequestDTO>,
) -> ApiResult<Json<ScanResultDTO>> {
    let result = state
        .access_control_service
        .scan(id, &body.token, body.direction.into(), actor(&user))
        .await?;
    Ok(Json(result.into()))
}

#[debug_handler]
async fn manual_entry(
    Extension(user): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<ManualEntryRequestDTO>,
) -> ApiResult<Json<ScanResultDTO>> {
    let result = state
        .access_control_service
        .record_manual(id, body.user_id, body.direction.into(), actor(&user))
        .await?;
    Ok(Json(result.into()))
}

#[debug_handler]
async fn search_members(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<SearchQuery>,
) -> ApiResult<Json<Vec<DoorSearchResultDTO>>> {
    let results = state
        .access_control_service
        .search_members(id, &query.q)
        .await?;
    Ok(Json(results.into_iter().map(Into::into).collect()))
}

#[debug_handler]
async fn delete_entry(
    Extension(user): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Path((id, entry_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<()> {
    state
        .access_control_service
        .delete_entry(id, entry_id, actor(&user))
        .await?;
    Ok(())
}

#[debug_handler]
async fn take_snapshot(
    Extension(user): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<TakeSnapshotDTO>,
) -> ApiResult<Json<AccessSnapshotDTO>> {
    let snapshot = state
        .access_control_service
        .take_snapshot(id, body.label, actor(&user))
        .await?;
    Ok(Json(snapshot.into()))
}

#[debug_handler]
async fn export_event(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response<Body>> {
    let service = &state.access_control_service;
    let (event, bytes) = service.export(id).await?;
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", service.export_content_type())
        .header(
            "Content-Disposition",
            format!(
                "attachment; filename=\"kulunvalvonta-{}.{}\"",
                event.event_date,
                service.export_file_extension()
            ),
        )
        .body(Body::from(bytes))
        .map_err(|_| ApiError::InternalServerError)
}

#[debug_handler]
async fn get_member_pass(
    Extension(user): Extension<Option<AuthenticatedUser>>,
    State(state): State<AppState>,
) -> ApiResult<Json<MemberAccessPassDTO>> {
    let user = user.ok_or(ApiError::Unauthorized)?;
    let pass = state
        .access_control_service
        .member_pass(user.user_id)
        .await?;
    let can_control = state.authentication_service.is_access_controller(&user);
    Ok(Json(MemberAccessPassDTO::new(pass, can_control)))
}
