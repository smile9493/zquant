-- Jobs table
CREATE TABLE jobs (
    id BIGSERIAL PRIMARY KEY,
    job_id VARCHAR(255) NOT NULL UNIQUE,
    job_type VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'queued',
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    progress JSONB,
    error JSONB,
    artifacts JSONB,
    executor_id VARCHAR(255),
    stop_requested BOOLEAN NOT NULL DEFAULT false,
    lease_until_ms BIGINT,
    version INTEGER NOT NULL DEFAULT 1,
    priority INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for claim query optimization
CREATE INDEX idx_jobs_claim ON jobs (status, priority DESC, created_at ASC);

-- Index for job_type queries
CREATE INDEX idx_jobs_type ON jobs (job_type);

-- Jobs idempotency table
CREATE TABLE jobs_idempotency (
    idempotency_key VARCHAR(255) PRIMARY KEY,
    job_id VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (job_id) REFERENCES jobs(job_id) ON DELETE CASCADE
);
