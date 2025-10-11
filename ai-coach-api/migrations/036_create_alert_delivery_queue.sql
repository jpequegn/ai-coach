-- Create alert delivery queue for robust notification delivery with retry logic
-- Migration: 036_create_alert_delivery_queue

-- Alert delivery queue table
CREATE TABLE alert_delivery_queue (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    alert_id UUID NOT NULL REFERENCES recovery_alerts(id) ON DELETE CASCADE,
    delivery_method VARCHAR(20) NOT NULL CHECK (delivery_method IN ('push', 'email', 'sms')),
    recipient_id VARCHAR(255) NOT NULL, -- device token, email address, or phone number
    attempts INT DEFAULT 0 NOT NULL,
    max_attempts INT DEFAULT 3 NOT NULL,
    last_attempt_at TIMESTAMPTZ,
    next_retry_at TIMESTAMPTZ,
    status VARCHAR(20) DEFAULT 'pending' NOT NULL CHECK (status IN ('pending', 'delivered', 'failed', 'cancelled')),
    error_message TEXT,
    delivered_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW() NOT NULL
);

-- Indexes for efficient queries
CREATE INDEX idx_alert_queue_status_retry ON alert_delivery_queue(status, next_retry_at)
    WHERE status = 'pending';

CREATE INDEX idx_alert_queue_alert ON alert_delivery_queue(alert_id);

CREATE INDEX idx_alert_queue_created ON alert_delivery_queue(created_at DESC);

CREATE INDEX idx_alert_queue_failed ON alert_delivery_queue(status, created_at DESC)
    WHERE status = 'failed';

-- Trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_alert_delivery_queue_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_alert_delivery_queue_updated_at_trigger
    BEFORE UPDATE ON alert_delivery_queue
    FOR EACH ROW
    EXECUTE FUNCTION update_alert_delivery_queue_updated_at();

-- Comments for documentation
COMMENT ON TABLE alert_delivery_queue IS 'Queue for alert notification delivery with automatic retry logic';
COMMENT ON COLUMN alert_delivery_queue.alert_id IS 'Reference to the recovery alert being delivered';
COMMENT ON COLUMN alert_delivery_queue.delivery_method IS 'Delivery method: push, email, or sms';
COMMENT ON COLUMN alert_delivery_queue.recipient_id IS 'Recipient identifier (device token, email, or phone)';
COMMENT ON COLUMN alert_delivery_queue.attempts IS 'Number of delivery attempts made';
COMMENT ON COLUMN alert_delivery_queue.max_attempts IS 'Maximum number of delivery attempts before marking as failed';
COMMENT ON COLUMN alert_delivery_queue.last_attempt_at IS 'Timestamp of last delivery attempt';
COMMENT ON COLUMN alert_delivery_queue.next_retry_at IS 'Timestamp when next retry should be attempted';
COMMENT ON COLUMN alert_delivery_queue.status IS 'Current delivery status: pending, delivered, failed, cancelled';
COMMENT ON COLUMN alert_delivery_queue.error_message IS 'Error message from last failed attempt';
COMMENT ON COLUMN alert_delivery_queue.delivered_at IS 'Timestamp when successfully delivered';
