-- Create job_execution_log table for tracking background job executions
-- Migration: 034_create_job_execution_log

-- Job execution log table
CREATE TABLE job_execution_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    job_name VARCHAR(100) NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    status VARCHAR(20) NOT NULL CHECK (status IN ('running', 'success', 'failed', 'cancelled')),
    records_processed INT DEFAULT 0,
    records_failed INT DEFAULT 0,
    error_message TEXT,
    execution_time_ms BIGINT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for efficient queries
CREATE INDEX idx_job_log_name_started ON job_execution_log(job_name, started_at DESC);
CREATE INDEX idx_job_log_status ON job_execution_log(status, started_at DESC);
CREATE INDEX idx_job_log_completed ON job_execution_log(completed_at DESC) WHERE completed_at IS NOT NULL;
CREATE INDEX idx_job_log_failed ON job_execution_log(job_name, status, started_at DESC) WHERE status = 'failed';

-- Trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_job_execution_log_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_job_execution_log_updated_at_trigger
    BEFORE UPDATE ON job_execution_log
    FOR EACH ROW
    EXECUTE FUNCTION update_job_execution_log_updated_at();

-- Comments for documentation
COMMENT ON TABLE job_execution_log IS 'Tracks execution history and status of background jobs';
COMMENT ON COLUMN job_execution_log.job_name IS 'Unique identifier for the job type';
COMMENT ON COLUMN job_execution_log.status IS 'Current status: running, success, failed, cancelled';
COMMENT ON COLUMN job_execution_log.records_processed IS 'Number of records successfully processed';
COMMENT ON COLUMN job_execution_log.records_failed IS 'Number of records that failed during processing';
COMMENT ON COLUMN job_execution_log.execution_time_ms IS 'Total execution time in milliseconds';
COMMENT ON COLUMN job_execution_log.metadata IS 'Additional job-specific metadata in JSON format';
