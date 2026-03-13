use metrics::{counter, histogram};

pub fn record_ingest_result(decision: String) {
    counter!("pipeline_ingest_total", "decision" => decision).increment(1);
}

pub fn record_stage_error(stage: String) {
    counter!("pipeline_stage_errors_total", "stage" => stage).increment(1);
}

pub fn record_stage_duration(stage: String, duration_secs: f64) {
    histogram!("pipeline_stage_duration_seconds", "stage" => stage).record(duration_secs);
}

pub fn record_route_fallback() {
    counter!("pipeline_route_fallback_total").increment(1);
}
