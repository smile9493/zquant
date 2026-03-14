use axum::body::Body;
use axum::http::{Request, StatusCode};
use job_application::api::ApiState;
use job_events::bus::InMemoryEventBus;
use job_store_pg::JobStore;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

async fn setup_state() -> ApiState {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".to_string());

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // Clean up test data to ensure test isolation
    sqlx::query("DELETE FROM jobs_idempotency")
        .execute(&pool)
        .await
        .expect("Failed to clean idempotency table");

    sqlx::query("DELETE FROM jobs")
        .execute(&pool)
        .await
        .expect("Failed to clean jobs table");

    ApiState {
        store: Arc::new(JobStore::new(pool)),
        bus: Arc::new(InMemoryEventBus::new(100)),
    }
}

#[tokio::test]
async fn test_list_jobs() {
    let state = setup_state().await;
    let app = job_application::api::router(state.clone());

    // Create a test job first
    let create_req = json!({
        "job_type": "test_job",
        "payload": {"key": "value"}
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs")
                .header("content-type", "application/json")
                .body(Body::from(create_req.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Now list jobs
    let response = app
        .oneshot(Request::builder().uri("/jobs").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify response contains JobSummary fields
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let jobs: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();

    assert!(!jobs.is_empty());
    let job = &jobs[0];
    assert!(job.get("job_id").is_some());
    assert!(job.get("job_type").is_some());
    assert!(job.get("status").is_some());
    assert!(job.get("stop_requested").is_some());
    assert!(job.get("created_at").is_some());
    assert!(job.get("updated_at").is_some());
}

#[tokio::test]
async fn test_stop_job_404() {
    let state = setup_state().await;
    let app = job_application::api::router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/nonexistent/stop")
                .header("content-type", "application/json")
                .body(Body::from(json!({"reason": "test"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_retry_job_404() {
    let state = setup_state().await;
    let app = job_application::api::router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs/nonexistent/retry")
                .body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_retry_job_success() {
    let state = setup_state().await;
    let app = job_application::api::router(state.clone());

    // Create original job
    let create_req = json!({
        "job_type": "test_job",
        "payload": {"key": "value"}
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs")
                .header("content-type", "application/json")
                .body(Body::from(create_req.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let original_job_id = create_response["job_id"].as_str().unwrap();

    // Retry the job
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/jobs/{}/retry", original_job_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let retry_response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let new_job_id = retry_response["job_id"].as_str().unwrap();

    // Verify new job has different ID
    assert_ne!(new_job_id, original_job_id);

    // Get new job details
    let response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/jobs/{}", new_job_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let new_job: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Verify job_type and payload match original
    assert_eq!(new_job["job_type"].as_str().unwrap(), "test_job");
    assert_eq!(new_job["payload"]["key"].as_str().unwrap(), "value");
    assert_eq!(new_job["status"].as_str().unwrap(), "queued");
}

#[tokio::test]
async fn test_get_health() {
    let state = setup_state().await;
    let app = job_application::api::router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/system/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let health: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(health["status"].as_str().unwrap(), "healthy");
    assert_eq!(health["mode"].as_str().unwrap(), "research");
    assert!(health.get("last_error").is_some());
}

#[tokio::test]
async fn test_get_job_logs_404() {
    let state = setup_state().await;
    let app = job_application::api::router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/jobs/nonexistent/logs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_job_logs_empty() {
    let state = setup_state().await;
    let app = job_application::api::router(state.clone());

    // Create a test job
    let create_req = json!({
        "job_type": "test_job",
        "payload": {"key": "value"}
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs")
                .header("content-type", "application/json")
                .body(Body::from(create_req.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let job_id = create_response["job_id"].as_str().unwrap();

    // Get logs for the job
    let response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/jobs/{}/logs", job_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let logs: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();

    // Should return empty array (no log collection yet)
    assert!(logs.is_empty());
}
