-- Pending Discord handles until first OAuth binds them to a snowflake row.
CREATE TABLE IF NOT EXISTS discord_pending_handles (
    handle      TEXT PRIMARY KEY NOT NULL,
    is_admin    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    created_by  TEXT NOT NULL DEFAULT 'admin'
);

-- Move leftover Option-A provisional keys (u.<handle>) out of discord_allowlist.
INSERT OR IGNORE INTO discord_pending_handles (handle, is_admin, created_at, created_by)
SELECT lower(COALESCE(username, substr(discord_id, 3))), is_admin, created_at, created_by
FROM discord_allowlist
WHERE discord_id LIKE 'u.%';

DELETE FROM discord_allowlist WHERE discord_id LIKE 'u.%';
