use axum::{
    debug_handler,
    extract::{Path, State},
    response::Redirect,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::repositories::application::{
    Application, ApplicationTargetableRole, NewApplication,
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/applications/targetable-roles", get(get_targetable_roles))
        .route("/applications", post(post_application))
        .route("/applications/:application_id", get(get_application))
        .with_state(state)
}

#[derive(Deserialize, Debug)]
struct ApplicationPath {
    application_id: Uuid,
}

async fn get_application(
    Path(path): Path<ApplicationPath>,
    State(state): State<AppState>,
) -> Result<Json<Application>, String> {
    let application_id = path.application_id;

    let application = state
        .application_service
        .get_application(application_id)
        .await;

    if let Err(e) = &application {
        println!("Error fetching application: {:?}", e);
    }

    application.map(Json).map_err(|e| e.to_string())
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

async fn post_application(
    State(state): State<AppState>,
    Json(new_application): Json<NewApplication>,
) -> Result<Redirect, String> {
    let application = state
        .application_service
        .create_application(new_application.clone())
        .await;

    let targetable_role = state
        .application_service
        .get_targetable_role(new_application.role_name, new_application.valid_until)
        .await;

    if let Err(e) = &application {
        println!("Error creating application: {:?}", e);
    }

    match targetable_role.map(|t| t.payment_link).ok().flatten() {
        Some(link) => Ok(Redirect::to(
            format!(
                "{}?client_reference_id={}",
                link,
                application.unwrap().application_id
            )
            .as_str(),
        )),
        None => Ok(Redirect::to(
            format!(
                "{}/applications/{}",
                state.config.frontend_url,
                application.unwrap().application_id
            )
            .as_str(),
        )),
    }
}
