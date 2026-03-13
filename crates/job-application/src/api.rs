use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
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

#[derive(Deserialize)]
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

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/jobs", post(create_job))
        .route("/jobs/:id", get(get_job))
        .with_state(state)
}
