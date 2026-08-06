DROP INDEX IF EXISTS analog_ingest_jobs_user_order_idx;

CREATE UNIQUE INDEX IF NOT EXISTS analog_ingest_jobs_user_order_done_idx
    ON analog_ingest_jobs(user_id, order_number)
    WHERE status = 'done';
