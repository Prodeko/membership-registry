use axum::Router;

use super::middleware::{check_auth, check_permission};

use super::AppState;

mod applications;
mod audit_logs;
mod email_templates;
mod marketing_tags;
mod members;
mod roles;
mod saved_filters;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/members", members::router(state.clone()))
        .nest("/applications", applications::router(state.clone()))
        .nest("/roles", roles::router(state.clone()))
        .nest("/saved-filters", saved_filters::router(state.clone()))
        .nest("/audit-logs", audit_logs::router(state.clone()))
        .nest("/email-templates", email_templates::router(state.clone()))
        .nest("/marketing-tags", marketing_tags::router(state.clone()))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            check_permission,
        ))
        .layer(axum::middleware::from_fn_with_state(
            // Require login
            state.clone(),
            check_auth,
        ))
}
