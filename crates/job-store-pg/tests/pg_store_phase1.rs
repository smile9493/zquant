use anyhow::Result;
use job_domain::JobStatus;
use job_store_pg::JobStore;
use sqlx::PgPool;

static TEST_DB_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

async fn setup_test_db() -> Result<PgPool> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".to_string());

    let pool = PgPool::connect(&database_url).await?;

    // Clean up tables
    sqlx::query("TRUNCATE TABLE jobs, jobs_idempotency CASCADE")
        .execute(&pool)
        .await?;

    Ok(pool)
}

#[tokio::test(flavor = "multi_thread")]
async fn create_job_no_idempotency_creates_queued() -> Result<()> {
    let _guard = TEST_DB_LOCK.lock().unwrap();
    drop(_guard);
    let pool = setup_test_db().await?;
    let store = JobStore::new(pool);

    let job = store
        .create_job(
            "test_job".to_string(),
            serde_json::json!({"data": "test"}),
            0,
            None,
        )
        .await?;

    assert_eq!(job.status, JobStatus::Queued);
    assert_eq!(job.job_type, "test_job");
    assert!(!job.job_id.is_empty());

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn create_job_with_idempotency_is_deduped() -> Result<()> {
    let _guard = TEST_DB_LOCK.lock().unwrap();
    drop(_guard);
    let pool = setup_test_db().await?;
    let store = JobStore::new(pool);

    let job1 = store
        .create_job(
            "test_job".to_string(),
            serde_json::json!({"data": "test"}),
            0,
            Some("idempotency_key_1".to_string()),
        )
        .await?;

    let job2 = store
        .create_job(
            "test_job".to_string(),
            serde_json::json!({"data": "different"}),
            0,
            Some("idempotency_key_1".to_string()),
        )
        .await?;

    assert_eq!(job1.job_id, job2.job_id);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn claim_skips_stop_requested() -> Result<()> {
    let _guard = TEST_DB_LOCK.lock().unwrap();
    drop(_guard);
    let pool = setup_test_db().await?;
    let store = JobStore::new(pool);

    let _job = store
        .create_job("test_job".to_string(), serde_json::json!({}), 0, None)
        .await?;

    store
        .request_stop(&_job.job_id, Some("test stop".to_string()))
        .await?;

    let claimed = store
        .claim_jobs("executor_1", 60000, 10, &["test_job".to_string()])
        .await?;

    assert!(claimed.is_empty());

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn claim_increments_lease_version() -> Result<()> {
    let _guard = TEST_DB_LOCK.lock().unwrap();
    drop(_guard);
    let pool = setup_test_db().await?;
    let store = JobStore::new(pool);

    let job = store
        .create_job("test_job".to_string(), serde_json::json!({}), 0, None)
        .await?;

    assert_eq!(job.lease_version, 0);

    let claimed = store
        .claim_jobs("executor_1", 60000, 10, &["test_job".to_string()])
        .await?;

    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].lease_version, 1);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn heartbeat_requires_matching_lease_version() -> Result<()> {
    let _guard = TEST_DB_LOCK.lock().unwrap();
    drop(_guard);
    let pool = setup_test_db().await?;
    let store = JobStore::new(pool);

    let _job = store
        .create_job("test_job".to_string(), serde_json::json!({}), 0, None)
        .await?;

    let claimed = store
        .claim_jobs("executor_1", 60000, 10, &["test_job".to_string()])
        .await?;

    let claimed_job = &claimed[0];

    // Correct lease_version should succeed
    let success = store
        .heartbeat_job(
            &claimed_job.job_id,
            "executor_1",
            claimed_job.lease_version,
            60000,
        )
        .await?;
    assert!(success);

    // Wrong lease_version should fail
    let failure = store
        .heartbeat_job(&claimed_job.job_id, "executor_1", 999, 60000)
        .await?;
    assert!(!failure);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn finalize_requires_matching_lease_version() -> Result<()> {
    let _guard = TEST_DB_LOCK.lock().unwrap();
    drop(_guard);
    let pool = setup_test_db().await?;
    let store = JobStore::new(pool);

    let _job1 = store
        .create_job("test_job".to_string(), serde_json::json!({}), 0, None)
        .await?;

    let _job2 = store
        .create_job("test_job".to_string(), serde_json::json!({}), 0, None)
        .await?;

    let claimed = store
        .claim_jobs("executor_1", 60000, 10, &["test_job".to_string()])
        .await?;

    // Wrong lease_version should fail
    let failure = store
        .finalize_job(
            &claimed[0].job_id,
            "executor_1",
            999,
            JobStatus::Done,
            None,
            None,
        )
        .await?;
    assert!(!failure);

    // Correct lease_version should succeed
    let success = store
        .finalize_job(
            &claimed[0].job_id,
            "executor_1",
            claimed[0].lease_version,
            JobStatus::Done,
            None,
            None,
        )
        .await?;
    assert!(success);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_claim_no_duplicates() -> Result<()> {
    let _guard = TEST_DB_LOCK.lock().unwrap();
    drop(_guard);
    let pool = setup_test_db().await?;
    let store1 = JobStore::new(pool.clone());
    let store2 = JobStore::new(pool);

    // Create 5 jobs
    for _ in 0..5 {
        store1
            .create_job("test_job".to_string(), serde_json::json!({}), 0, None)
            .await?;
    }

    // Concurrent claim from two executors
    let handle1 = tokio::spawn(async move {
        store1
            .claim_jobs("executor_1", 60000, 10, &["test_job".to_string()])
            .await
    });

    let handle2 = tokio::spawn(async move {
        store2
            .claim_jobs("executor_2", 60000, 10, &["test_job".to_string()])
            .await
    });

    let claimed1 = handle1.await??;
    let claimed2 = handle2.await??;

    // Total claimed should be 5
    assert_eq!(claimed1.len() + claimed2.len(), 5);

    // No duplicates
    let mut all_ids: Vec<String> = claimed1.iter().map(|j| j.job_id.clone()).collect();
    all_ids.extend(claimed2.iter().map(|j| j.job_id.clone()));
    all_ids.sort();
    all_ids.dedup();
    assert_eq!(all_ids.len(), 5);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_create_with_same_idempotency_key_returns_same_job() -> Result<()> {
    let _guard = TEST_DB_LOCK.lock().unwrap();
    drop(_guard);
    let pool = setup_test_db().await?;
    let store1 = JobStore::new(pool.clone());
    let store2 = JobStore::new(pool.clone());

    let handle1 = tokio::spawn(async move {
        store1
            .create_job(
                "test_job".to_string(),
                serde_json::json!({"request": 1}),
                0,
                Some("shared_idempotency_key".to_string()),
            )
            .await
    });

    let handle2 = tokio::spawn(async move {
        store2
            .create_job(
                "test_job".to_string(),
                serde_json::json!({"request": 2}),
                0,
                Some("shared_idempotency_key".to_string()),
            )
            .await
    });

    let job1 = handle1.await??;
    let job2 = handle2.await??;

    assert_eq!(job1.job_id, job2.job_id);

    let total_jobs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs")
        .fetch_one(&pool)
        .await?;
    let total_idempotency: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs_idempotency")
        .fetch_one(&pool)
        .await?;

    assert_eq!(total_jobs, 1);
    assert_eq!(total_idempotency, 1);

    Ok(())
}
