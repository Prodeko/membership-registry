use std::time::Duration;

use tokio_cron_scheduler::{JobBuilder, JobScheduler, JobSchedulerError};
use tokio_util::sync::CancellationToken;

use crate::application::services::{
    application_digest_service::ApplicationDigestService, renewal_service::RenewalService,
    role_group_service::RoleGroupService, role_service::RoleService,
};

const DAILY: Duration = Duration::from_secs(24 * 60 * 60);

pub async fn run_scheduler(
    role_service: RoleService,
    role_group_service: RoleGroupService,
    renewal_service: RenewalService,
    cancel: CancellationToken,
) {
    tracing::info!("Scheduler started — running initial tasks");

    run_cleanup(&role_service).await;
    run_group_cleanup(&role_group_service).await;
    run_renewals(&renewal_service).await;

    loop {
        tokio::select! {
            () = tokio::time::sleep(DAILY) => {
                run_cleanup(&role_service).await;
                run_group_cleanup(&role_group_service).await;
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

async fn run_group_cleanup(role_group_service: &RoleGroupService) {
    match role_group_service.cleanup_expired_group_memberships().await {
        Ok(count) => {
            tracing::info!(
                synced = count,
                "Scheduled expired group membership cleanup finished"
            );
        }
        Err(e) => {
            tracing::error!("Scheduled expired group membership cleanup failed: {e:?}");
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

/// Starts a cron scheduler with the single admin pending-application digest
/// job. Separate from the naive 24h loop above: the digest must fire at a
/// fixed local time, not at a startup-relative offset. Returns the running
/// scheduler; the caller shuts it down on exit.
pub async fn start_application_digest_job(
    digest_service: ApplicationDigestService,
    cron_expr: &str,
) -> Result<JobScheduler, JobSchedulerError> {
    let sched = JobScheduler::new().await?;
    let job = JobBuilder::new()
        .with_timezone(chrono_tz::Europe::Helsinki)
        .with_cron_job_type()
        .with_schedule(cron_expr)?
        .with_run_async(Box::new(move |_uuid, _lock| {
            let service = digest_service.clone();
            Box::pin(async move {
                tracing::info!("Running scheduled pending-application digest");
                service.send_pending_digest().await;
            })
        }))
        .build()?;
    sched.add(job).await?;
    sched.start().await?;
    tracing::info!(cron = cron_expr, "Application digest scheduler started");
    Ok(sched)
}
