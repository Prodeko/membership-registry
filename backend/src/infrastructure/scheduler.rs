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
/// job. Separate from `run_scheduler`'s naive 24h loop: the digest must fire
/// at a fixed local time, not at a startup-relative offset. Returns the
/// running scheduler; the caller shuts it down on exit.
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use envconfig::Envconfig;

    use super::*;
    use crate::application::services::tests::mocks::{
        MockApplicationQueryPort, MockAttributeRepositoryPort, MockRoleRepositoryPort,
    };
    use crate::config::Config;

    fn digest_service() -> ApplicationDigestService {
        ApplicationDigestService::new(
            Arc::new(MockApplicationQueryPort::new()),
            Arc::new(MockAttributeRepositoryPort::new()),
            Arc::new(MockRoleRepositoryPort::new()),
            None,
            "http://localhost".to_string(),
        )
    }

    /// The default `Config` produces via envconfig when APPLICATION_DIGEST_CRON
    /// is unset — the exact value production falls back to.
    fn default_cron_from_config() -> String {
        let required = [
            ("PORT", "8080"),
            ("DATABASE_URL", "postgres://unused"),
            ("FRONTEND_URL", "http://localhost"),
            ("KEYCLOAK_URL", "http://localhost"),
            ("KEYCLOAK_REALM", "realm"),
            ("KEYCLOAK_CLIENT_ID", "client"),
            ("KEYCLOAK_CLIENT_SECRET", "secret"),
            ("OAUTH_REDIRECT_URL", "http://localhost"),
            ("KEYCLOAK_ADMIN_CLIENT_ID", "admin-client"),
            ("KEYCLOAK_ADMIN_CLIENT_SECRET", "secret"),
            ("STRIPE_ENDPOINT_SECRET", "secret"),
        ];
        let env: HashMap<String, String> = required
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        Config::init_from_hashmap(&env)
            .unwrap()
            .application_digest_cron
    }

    #[tokio::test]
    async fn default_cron_expression_starts_the_scheduler() {
        let mut sched = start_application_digest_job(digest_service(), &default_cron_from_config())
            .await
            .expect("the default APPLICATION_DIGEST_CRON must be a valid 6-field cron expression");
        sched.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn malformed_cron_expression_is_an_error_not_a_panic() {
        let result = start_application_digest_job(digest_service(), "not a cron").await;
        assert!(result.is_err());
    }
}
