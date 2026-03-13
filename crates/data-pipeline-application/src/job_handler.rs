use async_trait::async_trait;
use data_pipeline_domain::IngestRequest;
use job_domain::{JobContext, JobHandler, JobResult};
use std::sync::Arc;

use crate::manager::DataPipelineManager;

pub struct DataPipelineJobHandler {
    manager: Arc<DataPipelineManager>,
}

impl DataPipelineJobHandler {
    pub fn new(manager: Arc<DataPipelineManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl JobHandler for DataPipelineJobHandler {
    fn job_types(&self) -> &'static [&'static str] {
        &["ingest_dataset"]
    }

    async fn handle(&self, ctx: JobContext) -> anyhow::Result<JobResult> {
        let req: IngestRequest = serde_json::from_value(ctx.payload)
            .map_err(|e| anyhow::anyhow!("failed to parse IngestRequest from job payload: {}", e))?;

        let result = self.manager.ingest_dataset(req).await
            .map_err(|e| anyhow::anyhow!("failed to ingest dataset: {}", e))?;

        let artifacts = serde_json::to_value(&result)
            .map_err(|e| anyhow::anyhow!("failed to serialize IngestResult: {}", e))?;

        Ok(JobResult {
            artifacts: Some(artifacts),
        })
    }
}
