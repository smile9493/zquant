use axum::{body::Body, http::Request};
use job_application::api::{router, ApiState};
use job_events::bus::InMemoryEventBus;
use job_store_pg::JobStore;
use serde_json::Value;
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

async fn setup_app() -> axum::Router {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".to_string());
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database");
    let state = ApiState {
        store: Arc::new(JobStore::new(pool)),
        bus: Arc::new(InMemoryEventBus::new(100)),
    };
    router(state)
}

#[tokio::test]
async fn test_get_datasources_shape() {
    let app = setup_app().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/datasources")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let arr = json.as_array().unwrap();
    assert!(!arr.is_empty());
    let first = &arr[0];
    assert!(first.get("id").is_some());
    assert!(first.get("name").is_some());
}

#[tokio::test]
async fn test_get_datasets_shape() {
    let app = setup_app().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/datasets")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let arr = json.as_array().unwrap();
    assert!(!arr.is_empty());
    let first = &arr[0];
    assert!(first.get("id").is_some());
    assert!(first.get("name").is_some());
    assert!(first.get("source_id").is_some());
}
