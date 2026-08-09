-- Discord login allowlist (fail closed). is_admin grants /admin without password.
CREATE TABLE IF NOT EXISTS discord_allowlist (
    discord_id  TEXT PRIMARY KEY NOT NULL,
    username    TEXT,
    is_admin    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    created_by  TEXT NOT NULL DEFAULT 'admin'
);
