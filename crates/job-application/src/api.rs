use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use tower_http::cors::CorsLayer;
use chrono::{DateTime, Utc};
use job_events::{
    bus::{Event, EventBus},
    types::JobCreated,
};
use job_store_pg::JobStore;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct ApiState {
    pub store: Arc<JobStore>,
    pub bus: Arc<dyn EventBus>,
}

#[derive(Deserialize, Serialize)]
pub struct CreateJobRequest {
    pub job_type: String,
    pub payload: serde_json::Value,
    #[serde(default)]
    pub priority: Option<i32>,
    pub idempotency_key: Option<String>,
}

#[derive(Serialize)]
pub struct CreateJobResponse {
    pub job_id: String,
}

#[derive(Serialize)]
pub struct JobSummary {
    pub job_id: String,
    pub job_type: String,
    pub status: String,
    pub stop_requested: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct StopJobRequest {
    pub reason: Option<String>,
}

#[derive(Serialize)]
pub struct RetryJobResponse {
    pub job_id: String,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub mode: String,
    pub last_error: Option<String>,
}

#[derive(Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct DataSource {
    pub id: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct DataSet {
    pub id: String,
    pub name: String,
    pub source_id: String,
}

async fn create_job(
    State(state): State<ApiState>,
    Json(req): Json<CreateJobRequest>,
) -> Result<Json<CreateJobResponse>, StatusCode> {
    let job = state
        .store
        .create_job(
            req.job_type.clone(),
            req.payload,
            req.priority.unwrap_or(0),
            req.idempotency_key,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    state.bus.publish(Event::JobCreated(JobCreated {
        job_id: job.job_id.clone(),
        job_type: req.job_type.clone(),
        created_at: Utc::now(),
    }));

    tracing::info!(
        job_id = %job.job_id,
        job_type = %req.job_type,
        "Job created"
    );

    Ok(Json(CreateJobResponse { job_id: job.job_id }))
}

async fn get_job(
    State(state): State<ApiState>,
    Path(job_id): Path<String>,
) -> Result<Json<job_domain::Job>, StatusCode> {
    let job = state
        .store
        .get_job(&job_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(job))
}

async fn list_jobs(
    State(state): State<ApiState>,
) -> Result<Json<Vec<JobSummary>>, StatusCode> {
    let jobs = state
        .store
        .list_jobs()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let summaries = jobs
        .into_iter()
        .map(|job| {
            let status = match job.status {
                job_domain::JobStatus::Queued => "queued",
                job_domain::JobStatus::Running => "running",
                job_domain::JobStatus::Done => "done",
                job_domain::JobStatus::Error => "error",
                job_domain::JobStatus::Stopped => "stopped",
                job_domain::JobStatus::Reaped => "reaped",
            };
            JobSummary {
                job_id: job.job_id,
                job_type: job.job_type,
                status: status.to_string(),
                stop_requested: job.stop_requested,
                created_at: job.created_at,
                updated_at: job.updated_at,
            }
        })
        .collect();

    Ok(Json(summaries))
}

async fn stop_job(
    State(state): State<ApiState>,
    Path(job_id): Path<String>,
    Json(req): Json<StopJobRequest>,
) -> Result<StatusCode, StatusCode> {
    // Check if job exists
    let job_exists = state
        .store
        .get_job(&job_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_some();

    if !job_exists {
        return Err(StatusCode::NOT_FOUND);
    }

    state
        .store
        .request_stop(&job_id, req.reason)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

async fn retry_job(
    State(state): State<ApiState>,
    Path(job_id): Path<String>,
) -> Result<Json<RetryJobResponse>, StatusCode> {
    let original_job = state
        .store
        .get_job(&job_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let new_job = state
        .store
        .create_job(
            original_job.job_type,
            original_job.payload,
            0,
            None,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RetryJobResponse {
        job_id: new_job.job_id,
    }))
}

async fn get_health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        mode: "research".to_string(),
        last_error: None,
    })
}

async fn get_job_logs(
    State(state): State<ApiState>,
    Path(job_id): Path<String>,
) -> Result<Json<Vec<LogEntry>>, StatusCode> {
    // Check if job exists
    let job_exists = state
        .store
        .get_job(&job_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_some();

    if !job_exists {
        return Err(StatusCode::NOT_FOUND);
    }

    // Return empty logs for now (log collection to be implemented later)
    Ok(Json(vec![]))
}

async fn get_datasources() -> Json<Vec<DataSource>> {
    Json(vec![
        DataSource {
            id: "yahoo".to_string(),
            name: "Yahoo Finance".to_string(),
        },
        DataSource {
            id: "alphavantage".to_string(),
            name: "Alpha Vantage".to_string(),
        },
    ])
}

async fn get_datasets() -> Json<Vec<DataSet>> {
    Json(vec![
        DataSet {
            id: "us_stocks".to_string(),
            name: "US Stocks".to_string(),
            source_id: "yahoo".to_string(),
        },
        DataSet {
            id: "crypto".to_string(),
            name: "Cryptocurrencies".to_string(),
            source_id: "yahoo".to_string(),
        },
    ])
}

pub fn router(state: ApiState) -> Router {
    let ws_state = crate::ws::WsState {
        store: state.store.clone(),
        bus: state.bus.clone(),
    };

    Router::new()
        .route("/ws", get(crate::ws::ws_handler))
        .with_state(ws_state)
        .route("/system/health", get(get_health))
        .route("/jobs", post(create_job).get(list_jobs))
        .route("/jobs/:id", get(get_job))
        .route("/jobs/:id/stop", post(stop_job))
        .route("/jobs/:id/retry", post(retry_job))
        .route("/jobs/:id/logs", get(get_job_logs))
        .route("/api/datasources", get(get_datasources))
        .route("/api/datasets", get(get_datasets))
        .with_state(state)
        .layer(CorsLayer::permissive())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_summary_serialization() {
        let summary = JobSummary {
            job_id: "job_123".to_string(),
            job_type: "test".to_string(),
            status: "queued".to_string(),
            stop_requested: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("job_123"));
        assert!(json.contains("queued"));
    }

    #[test]
    fn test_create_job_request_deserialization() {
        let json = r#"{"job_type":"test","payload":{"key":"value"}}"#;
        let req: CreateJobRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.job_type, "test");
        assert_eq!(req.priority, None);
    }

    // Note: Full API integration tests for GET /jobs, POST /jobs/:id/stop,
    // and POST /jobs/:id/retry require a PostgreSQL database environment.
    // These tests verify:
    // 1. GET /jobs returns job list with correct JobSummary format
    // 2. POST /jobs/:id/stop returns 404 for non-existent jobs
    // 3. POST /jobs/:id/retry returns 404 for non-existent jobs
    // 4. POST /jobs/:id/retry creates a new job with same type and payload
}
