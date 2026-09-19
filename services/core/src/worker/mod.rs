mod document_title;
mod embed_job;
mod extract_job;
mod render_job;

use std::time::Duration;

use crate::{AppError, AppResult, AppState, models::ProcessingJob, pdf::PdfEngine};

const INITIAL_FAILURE_BACKOFF: Duration = Duration::from_secs(2);
const MAX_FAILURE_BACKOFF: Duration = Duration::from_secs(30);

pub struct Worker {
    state: AppState,
    pdf: PdfEngine,
    worker_id: String,
}

impl Worker {
    pub fn new(state: AppState) -> Self {
        let worker_id = format!("{}-{}", hostname(), uuid::Uuid::new_v4().simple());
        let pdf = PdfEngine::new(
            state.config.pdfium_lib_path.clone(),
            state.config.unicode_font_path.clone(),
            state.config.max_document_bytes,
            state.config.max_document_pages,
        );
        Self {
            state,
            pdf,
            worker_id,
        }
    }

    pub async fn extract_document(&self, job: &ProcessingJob) -> AppResult<()> {
        extract_job::run(&self.state, &self.pdf, job).await
    }

    pub async fn run(self) -> AppResult<()> {
        tracing::info!(worker_id = %self.worker_id, "document worker started");
        let mut failure_backoff = INITIAL_FAILURE_BACKOFF;
        loop {
            tokio::select! {
                _ = crate::shutdown_signal() => {
                    tracing::info!("document worker stopping");
                    return Ok(());
                }
                result = self.run_once() => {
                    match result {
                        Ok(true) => failure_backoff = INITIAL_FAILURE_BACKOFF,
                        Ok(false) => {
                            failure_backoff = INITIAL_FAILURE_BACKOFF;
                            tokio::time::sleep(Duration::from_millis(750)).await;
                        }
                        Err(error) => {
                            tracing::error!(
                                ?error,
                                retry_in_seconds = failure_backoff.as_secs(),
                                "worker loop failed"
                            );
                            tokio::time::sleep(failure_backoff).await;
                            failure_backoff = next_failure_backoff(failure_backoff);
                        }
                    }
                }
            }
        }
    }

    pub async fn run_once(&self) -> AppResult<bool> {
        let Some(job) = self.claim_job().await? else {
            return Ok(false);
        };
        let result = match job.kind.as_str() {
            "extract" | "map" => extract_job::run(&self.state, &self.pdf, &job).await,
            "render" => render_job::run(&self.state, &self.pdf, &job).await,
            "embed" => embed_job::run(&self.state, &job).await,
            "delete" => embed_job::delete_memory(&self.state, &job).await,
            _ => Err(AppError::Validation("Unknown processing job".to_owned())),
        };

        match result {
            Ok(()) => self.finish_job(job.id).await?,
            Err(error) => self.fail_job(&job, &error).await?,
        }
        Ok(true)
    }

    async fn claim_job(&self) -> AppResult<Option<ProcessingJob>> {
        sqlx::query_as::<_, ProcessingJob>(
            "with next_job as (
                select id from public.processing_jobs
                where (status = 'queued' and run_after <= now())
                   or (status = 'running' and locked_at < now() - interval '15 minutes')
                order by created_at
                for update skip locked
                limit 1
             )
             update public.processing_jobs j
             set status = 'running', locked_at = now(), locked_by = $1,
                 attempts = attempts + 1
             from next_job
             where j.id = next_job.id
             returning j.id, j.document_id, j.owner_id, j.kind,
                       j.status::text as status, j.payload, j.attempts",
        )
        .bind(&self.worker_id)
        .fetch_optional(&self.state.pool)
        .await
        .map_err(Into::into)
    }

    async fn finish_job(&self, job_id: uuid::Uuid) -> AppResult<()> {
        sqlx::query(
            "update public.processing_jobs
             set status = 'succeeded', locked_at = null, locked_by = null
             where id = $1",
        )
        .bind(job_id)
        .execute(&self.state.pool)
        .await?;
        Ok(())
    }

    async fn fail_job(&self, job: &ProcessingJob, error: &AppError) -> AppResult<()> {
        let retryable = job.attempts < 3
            && !matches!(
                error,
                AppError::Validation(_) | AppError::Configuration(_) | AppError::NotFound
            );
        let status = if retryable { "queued" } else { "failed" };
        let run_after_seconds = i32::from(job.attempts.max(1)) * 30;
        let error_code = error_code(error);
        let mut transaction = self.state.pool.begin().await?;
        sqlx::query(
            "update public.processing_jobs
             set status = $2::public.job_status, run_after = now() + make_interval(secs => $3),
                 locked_at = null, locked_by = null, error_code = $4
             where id = $1",
        )
        .bind(job.id)
        .bind(status)
        .bind(run_after_seconds)
        .bind(error_code)
        .execute(&mut *transaction)
        .await?;
        if !retryable {
            sqlx::query(
                "update public.documents
                 set status = 'failed', error_code = $2
                 where id = $1",
            )
            .bind(job.document_id)
            .bind(error_code)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        tracing::warn!(
            job_id = %job.id,
            document_id = %job.document_id,
            retryable,
            ?error,
            "processing job failed"
        );
        Ok(())
    }
}

fn error_code(error: &AppError) -> &'static str {
    match error {
        AppError::Validation(_) => "invalid_document",
        AppError::Configuration(_) => "service_not_configured",
        AppError::Upstream => "upstream_unavailable",
        AppError::NotFound => "document_missing",
        _ => "processing_failed",
    }
}

fn hostname() -> String {
    std::env::var("HOSTNAME").unwrap_or_else(|_| "worker".to_owned())
}

fn next_failure_backoff(current: Duration) -> Duration {
    current.saturating_mul(2).min(MAX_FAILURE_BACKOFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_original_is_reported_as_such() {
        assert_eq!(error_code(&AppError::NotFound), "document_missing");
        assert_eq!(error_code(&AppError::Upstream), "upstream_unavailable");
    }

    #[test]
    fn worker_failure_backoff_is_capped() {
        assert_eq!(
            next_failure_backoff(INITIAL_FAILURE_BACKOFF),
            Duration::from_secs(4)
        );
        assert_eq!(
            next_failure_backoff(Duration::from_secs(16)),
            MAX_FAILURE_BACKOFF
        );
        assert_eq!(
            next_failure_backoff(MAX_FAILURE_BACKOFF),
            MAX_FAILURE_BACKOFF
        );
    }
}
