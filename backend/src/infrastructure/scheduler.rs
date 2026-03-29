use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::application::services::{renewal_service::RenewalService, role_service::RoleService};

const DAILY: Duration = Duration::from_secs(24 * 60 * 60);

pub async fn run_scheduler(
    role_service: RoleService,
    renewal_service: RenewalService,
    cancel: CancellationToken,
) {
    tracing::info!("Scheduler started — running initial tasks");

    run_cleanup(&role_service).await;
    run_renewals(&renewal_service).await;

    loop {
        tokio::select! {
            () = tokio::time::sleep(DAILY) => {
                run_cleanup(&role_service).await;
                run_renewals(&renewal_service).await;
            }
            () = cancel.cancelled() => {
                tracing::info!("Scheduler shutting down");
                return;
            }
        }
    }
}

async fn run_cleanup(role_service: &RoleService) {
    match role_service.cleanup_expired_roles().await {
        Ok(count) => {
            tracing::info!(synced = count, "Scheduled expired role cleanup finished");
        }
        Err(e) => {
            tracing::error!("Scheduled expired role cleanup failed: {e:?}");
        }
    }
}

async fn run_renewals(renewal_service: &RenewalService) {
    match renewal_service.process_pending_renewals().await {
        Ok(()) => {
            tracing::info!("Scheduled renewal processing finished");
        }
        Err(e) => {
            tracing::error!("Scheduled renewal processing failed: {e:?}");
        }
    }
}
