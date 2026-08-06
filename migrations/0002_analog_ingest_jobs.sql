CREATE TABLE IF NOT EXISTS analog_ingest_jobs (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id       INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    order_number  TEXT    NOT NULL,
    secure_id     TEXT,
    camera_label  TEXT    NOT NULL,
    album_name    TEXT,
    status        TEXT    NOT NULL DEFAULT 'queued',
    error_text    TEXT,
    created_at    TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS analog_ingest_jobs_user_id_idx ON analog_ingest_jobs(user_id);
CREATE INDEX IF NOT EXISTS analog_ingest_jobs_status_idx ON analog_ingest_jobs(status);
CREATE UNIQUE INDEX IF NOT EXISTS analog_ingest_jobs_user_order_done_idx
    ON analog_ingest_jobs(user_id, order_number)
    WHERE status = 'done';
