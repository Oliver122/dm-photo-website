-- SQLite forbids non-constant defaults (datetime('now')) in ALTER TABLE ADD
-- COLUMN, so these are added nullable and backfilled; the application always
-- writes them explicitly going forward.
ALTER TABLE tickets ADD COLUMN last_updated TEXT;
ALTER TABLE tickets ADD COLUMN completed_at TEXT;

UPDATE tickets SET last_updated = created_at WHERE last_updated IS NULL;

CREATE INDEX IF NOT EXISTS tickets_completed_at_idx ON tickets(completed_at);
