use anyhow::Result;
use chrono::{DateTime, Utc};
use job_domain::{Job, JobStatus};
use sqlx::{FromRow, PgPool};

fn generate_job_id() -> String {
    format!("job_{}", uuid::Uuid::new_v4().simple())
}

#[derive(FromRow)]
struct JobRow {
    #[allow(dead_code)]
    id: i64,
    job_id: String,
    job_type: String,
    status: String,
    payload: serde_json::Value,
    progress: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
    artifacts: Option<serde_json::Value>,
    executor_id: Option<String>,
    stop_requested: bool,
    stop_reason: Option<String>,
    lease_until_ms: Option<i64>,
    lease_version: i64,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<JobRow> for Job {
    fn from(row: JobRow) -> Self {
        let status = match row.status.as_str() {
            "queued" => JobStatus::Queued,
            "running" => JobStatus::Running,
            "done" => JobStatus::Done,
            "error" => JobStatus::Error,
            "stopped" => JobStatus::Stopped,
            "reaped" => JobStatus::Reaped,
            _ => JobStatus::Error,
        };

        Job {
            job_id: row.job_id,
            job_type: row.job_type,
            status,
            payload: row.payload,
            progress: row.progress,
            error: row.error,
            artifacts: row.artifacts,
            executor_id: row.executor_id,
            stop_requested: row.stop_requested,
            stop_reason: row.stop_reason,
            lease_until_ms: row.lease_until_ms,
            lease_version: row.lease_version,
            version: row.version,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}


pub struct JobStore {
    pool: PgPool,
}

impl JobStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new job with optional idempotency
    pub async fn create_job(
        &self,
        job_type: String,
        payload: serde_json::Value,
        priority: i32,
        idempotency_key: Option<String>,
    ) -> Result<Job> {
        let mut tx = self.pool.begin().await?;

        // Check idempotency if key provided
        if let Some(ref key) = idempotency_key {
            if let Some(existing_job_id) = sqlx::query_scalar::<_, String>(
                "SELECT job_id FROM jobs_idempotency WHERE idempotency_key = $1"
            )
            .bind(key)
            .fetch_optional(&mut *tx)
            .await?
            {
                // Return existing job
                let job = sqlx::query_as::<_, JobRow>(
                    "SELECT id, job_id, job_type, status, payload, progress, error,
                            artifacts, executor_id, stop_requested, stop_reason,
                            lease_until_ms, lease_version, version, created_at, updated_at
                     FROM jobs WHERE job_id = $1"
                )
                .bind(&existing_job_id)
                .fetch_one(&mut *tx)
                .await?;

                tx.commit().await?;
                return Ok(job.into());
            }
        }

        // Create new job
        let job_id = generate_job_id();
        let now = Utc::now();

        let job = sqlx::query_as::<_, JobRow>(
            "INSERT INTO jobs (job_id, job_type, status, payload, priority, created_at, updated_at)
             VALUES ($1, $2, 'queued', $3, $4, $5, $5)
             RETURNING id, job_id, job_type, status, payload, progress, error,
                       artifacts, executor_id, stop_requested, stop_reason,
                       lease_until_ms, lease_version, version, created_at, updated_at"
        )
        .bind(&job_id)
        .bind(&job_type)
        .bind(&payload)
        .bind(priority)
        .bind(now)
        .fetch_one(&mut *tx)
        .await?;

        // Insert idempotency record if key provided
        if let Some(key) = idempotency_key {
            let expires_at = now + chrono::Duration::days(7);
            sqlx::query(
                "INSERT INTO jobs_idempotency (idempotency_key, job_id, created_at, expires_at)
                 VALUES ($1, $2, $3, $4)"
            )
            .bind(&key)
            .bind(&job_id)
            .bind(now)
            .bind(expires_at)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(job.into())
    }

    /// Get job by job_id
    pub async fn get_job(&self, job_id: &str) -> Result<Option<Job>> {
        let job = sqlx::query_as::<_, JobRow>(
            "SELECT id, job_id, job_type, status, payload, progress, error,
                    artifacts, executor_id, stop_requested, stop_reason,
                    lease_until_ms, lease_version, version, created_at, updated_at
             FROM jobs WHERE job_id = $1"
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(job.map(Into::into))
    }

    /// Reap expired jobs (lease timeout)
    pub async fn reap_expired_jobs(&self, now_ms: i64, batch: i32) -> Result<Vec<Job>> {
        let jobs = sqlx::query_as::<_, JobRow>(
            "UPDATE jobs SET status = 'stopped', stop_reason = 'reaped',
             lease_until_ms = NULL, updated_at = $1
             WHERE id IN (
                 SELECT id FROM jobs
                 WHERE status = 'running' AND lease_until_ms < $2
                 LIMIT $3
                 FOR UPDATE SKIP LOCKED
             )
             RETURNING id, job_id, job_type, status, payload, progress, error,
                       artifacts, executor_id, stop_requested, stop_reason,
                       lease_until_ms, lease_version, version, created_at, updated_at"
        )
        .bind(Utc::now())
        .bind(now_ms)
        .bind(batch)
        .fetch_all(&self.pool)
        .await?;

        Ok(jobs.into_iter().map(Into::into).collect())
    }

    /// Claim jobs atomically using FOR UPDATE SKIP LOCKED
    pub async fn claim_jobs(
        &self,
        executor_id: &str,
        lease_duration_ms: i64,
        limit: i32,
        allowed_job_types: &[String],
    ) -> Result<Vec<Job>> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now();
        let lease_until = now.timestamp_millis() + lease_duration_ms;

        let job_ids: Vec<i64> = sqlx::query_scalar(
            "SELECT id FROM jobs
             WHERE status = 'queued' AND stop_requested = false
             AND job_type = ANY($1)
             ORDER BY priority DESC, created_at ASC
             FOR UPDATE SKIP LOCKED LIMIT $2"
        )
        .bind(allowed_job_types)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await?;

        if job_ids.is_empty() {
            tx.commit().await?;
            return Ok(vec![]);
        }

        let jobs = sqlx::query_as::<_, JobRow>(
            "UPDATE jobs SET status = 'running', executor_id = $1,
             lease_until_ms = $2, lease_version = lease_version + 1,
             version = version + 1, updated_at = $3
             WHERE id = ANY($4)
             RETURNING id, job_id, job_type, status, payload, progress, error,
                       artifacts, executor_id, stop_requested, stop_reason,
                       lease_until_ms, lease_version, version, created_at, updated_at"
        )
        .bind(executor_id)
        .bind(lease_until)
        .bind(now)
        .bind(&job_ids)
        .fetch_all(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(jobs.into_iter().map(Into::into).collect())
    }

    /// Finalize job with terminal status and fencing
    pub async fn finalize_job(
        &self,
        job_id: &str,
        executor_id: &str,
        lease_version: i64,
        terminal_status: JobStatus,
        artifacts: Option<serde_json::Value>,
        error: Option<serde_json::Value>,
    ) -> Result<bool> {
        let status_str = match terminal_status {
            JobStatus::Done => "done",
            JobStatus::Error => "error",
            JobStatus::Stopped => "stopped",
            JobStatus::Reaped => "reaped",
            _ => anyhow::bail!("Invalid terminal status"),
        };

        let result = sqlx::query(
            "UPDATE jobs SET status = $1, artifacts = $2, error = $3,
             lease_until_ms = NULL, updated_at = $4
             WHERE job_id = $5 AND status = 'running'
             AND executor_id = $6 AND lease_version = $7"
        )
        .bind(status_str)
        .bind(artifacts)
        .bind(error)
        .bind(Utc::now())
        .bind(job_id)
        .bind(executor_id)
        .bind(lease_version)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update job heartbeat and extend lease with fencing
    pub async fn heartbeat_job(
        &self,
        job_id: &str,
        executor_id: &str,
        lease_version: i64,
        lease_duration_ms: i64,
    ) -> Result<bool> {
        let now = Utc::now();
        let lease_until = now.timestamp_millis() + lease_duration_ms;

        let result = sqlx::query(
            "UPDATE jobs SET lease_until_ms = $1, updated_at = $2
             WHERE job_id = $3 AND status = 'running'
             AND executor_id = $4 AND lease_version = $5"
        )
        .bind(lease_until)
        .bind(now)
        .bind(job_id)
        .bind(executor_id)
        .bind(lease_version)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Request job to stop with reason
    pub async fn request_stop(&self, job_id: &str, reason: Option<String>) -> Result<()> {
        sqlx::query(
            "UPDATE jobs SET stop_requested = true, stop_reason = $1, updated_at = $2
             WHERE job_id = $3"
        )
        .bind(reason)
        .bind(Utc::now())
        .bind(job_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

