use axum::{
    debug_handler, extract::{Path, State}, routing::{delete, get, post, put}, Json, Router
};
use serde::Deserialize;
use uuid::Uuid;

use crate::repositories::application::{Application, ApplicationTargetableRole};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/applications", get(get_applications))
         // .route("/applications", post(post_application))
        .route("/applications/:application_id", get(get_application))
        .route("/applications/:application_id/status", put(update_application_status))
        .route("/applications/targetable-roles", get(post_targetable_role))
        .route("/applications/targetable-roles", post(post_targetable_role))
        .route("/applications/targetable-roles", put(put_targetable_role))
        .with_state(state)
}

async fn get_applications(State(state): State<AppState>) -> Result<Json<Vec<Application>>, String> {
    let applications = state.application_service.get_all_applications().await;

    if let Err(e) = &applications {
        println!("Error fetching applications: {:?}", e);
    }

    applications.map(Json).map_err(|e| e.to_string())
}

#[derive(Deserialize, Debug)]
struct ApplicationPath {
    application_id: String,
}

async fn get_application(
    Path(path): Path<ApplicationPath>,
    State(state): State<AppState>,
) -> Result<Json<Application>, String> {
    let application_id = Uuid::parse_str(&path.application_id).map_err(|e| e.to_string())?;

    let application = state
        .application_service
        .get_application(application_id)
        .await;

    if let Err(e) = &application {
        println!("Error fetching application: {:?}", e);
    }

    application.map(Json).map_err(|e| e.to_string())
}

#[derive(Deserialize, Debug)]
struct UpdateApplicationStatus {
    status: String,
}

async fn update_application_status(
    Path(path): Path<ApplicationPath>,
    State(state): State<AppState>,
    Json(body): Json<UpdateApplicationStatus>,
) -> Result<Json<()>, String> {
    let application_id = Uuid::parse_str(&path.application_id).map_err(|e| e.to_string())?;
    let status = body.status;

    let update = state
        .application_service
        .update_application_status(application_id, status)
        .await;

    if let Err(e) = &update {
        println!("Error updating application status: {:?}", e);
    }

    update.map(Json).map_err(|e| e.to_string())
}

#[debug_handler]
async fn get_targetable_roles(
    State(state): State<AppState>,
) -> Result<Json<Vec<ApplicationTargetableRole>>, String> {
    let targetable_roles = state.application_service.fetch_all_targetable_roles().await;

    if let Err(e) = &targetable_roles {
        println!("Error fetching targetable roles: {:?}", e);
    }

    targetable_roles.map(Json).map_err(|e| e.to_string())
}

#[derive(Deserialize, Debug)]
struct PostTargetableRole {
    role_name: String,
    valid_until: chrono::NaiveDate,
}
#[debug_handler]
async fn post_targetable_role(
    State(state): State<AppState>,
    Json(body): Json<PostTargetableRole>,
) -> Result<Json<()>, String> {
    let role_name = body.role_name;
    let valid_until = body.valid_until;

    let targetable_role = state
        .application_service
        .create_targetable_role(role_name, valid_until, Some(true))
        .await;

    if let Err(e) = &targetable_role {
        println!("Error creating targetable role: {:?}", e);
    }

    targetable_role.map(Json).map_err(|e| e.to_string())
}

#[derive(Deserialize, Debug)]
struct PutTargetableRole {
    role_name: String,
    valid_until: chrono::NaiveDate,
    active: bool,
}
#[debug_handler]
async fn put_targetable_role(
    State(state): State<AppState>,
    Json(body): Json<PutTargetableRole>,
) -> Result<Json<()>, String> {
    let role_name = body.role_name;
    let valid_until = body.valid_until;
    let active = body.active;

    let targetable_role = state
        .application_service
        .update_targetable_role(role_name, valid_until, Some(active))
        .await;

    if let Err(e) = &targetable_role {
        println!("Error updating targetable role: {:?}", e);
    }

    targetable_role.map(Json).map_err(|e| e.to_string())
}
