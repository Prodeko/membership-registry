use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::application::services::{
    marketing_sync_service::MarketingSyncService, renewal_service::RenewalService,
    role_service::RoleService,
};

const DAILY: Duration = Duration::from_secs(24 * 60 * 60);

pub async fn run_scheduler(
    role_service: RoleService,
    renewal_service: RenewalService,
    marketing_sync_service: MarketingSyncService,
    cancel: CancellationToken,
) {
    tracing::info!("Scheduler started — running initial tasks");

    run_cleanup(&role_service).await;
    run_renewals(&renewal_service).await;
    run_marketing_sync(&marketing_sync_service).await;

    loop {
        tokio::select! {
            () = tokio::time::sleep(DAILY) => {
                run_cleanup(&role_service).await;
                run_renewals(&renewal_service).await;
                run_marketing_sync(&marketing_sync_service).await;
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

async fn run_marketing_sync(marketing_sync_service: &MarketingSyncService) {
    match marketing_sync_service.run_sync().await {
        Ok(()) => {
            tracing::info!("Scheduled marketing list sync finished");
        }
        Err(e) => {
            tracing::error!("Scheduled marketing list sync failed: {e:?}");
        }
    }
}
