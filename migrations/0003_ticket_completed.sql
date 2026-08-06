ALTER TABLE tickets ADD COLUMN completed INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS tickets_completed_idx ON tickets(completed);
