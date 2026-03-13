-- Phase 1 migration: Add fencing, constraints, and optimized indexes

-- 1. Add new fields to jobs table
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS lease_version BIGINT NOT NULL DEFAULT 0;
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS stop_reason TEXT;
ALTER TABLE jobs ALTER COLUMN version SET DEFAULT 0;

-- 2. Add status constraint (allow all Phase 1 states)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'jobs_status_check'
    ) THEN
        ALTER TABLE jobs ADD CONSTRAINT jobs_status_check
            CHECK (status IN ('queued', 'running', 'done', 'error', 'stopped', 'reaped'));
    END IF;
END $$;

-- 3. Add running state constraints
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'jobs_running_requires_executor'
    ) THEN
        ALTER TABLE jobs ADD CONSTRAINT jobs_running_requires_executor
            CHECK (status != 'running' OR (executor_id IS NOT NULL AND lease_until_ms IS NOT NULL));
    END IF;
END $$;

-- 4. Drop old indexes and create optimized partial indexes
DROP INDEX IF EXISTS idx_jobs_claim;
DROP INDEX IF EXISTS idx_jobs_type;

-- Claim index: only index queued jobs that are not stopped
CREATE INDEX IF NOT EXISTS idx_jobs_claim_optimized ON jobs (priority DESC, created_at ASC)
    WHERE status = 'queued' AND stop_requested = false;

-- Lease sweep index: only index running jobs with lease
CREATE INDEX IF NOT EXISTS idx_jobs_lease_sweep ON jobs (lease_until_ms)
    WHERE status = 'running' AND lease_until_ms IS NOT NULL;

-- Stop sweep index: for future stop sweep implementation
CREATE INDEX IF NOT EXISTS idx_jobs_stop_sweep ON jobs (created_at)
    WHERE status = 'queued' AND stop_requested = true;

-- 5. Enhance jobs_idempotency table
ALTER TABLE jobs_idempotency ADD COLUMN IF NOT EXISTS expires_at TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '7 days');

-- Index for idempotency sweep
CREATE INDEX IF NOT EXISTS idx_jobs_idempotency_expires ON jobs_idempotency (expires_at);

