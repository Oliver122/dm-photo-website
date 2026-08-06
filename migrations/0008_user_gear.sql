CREATE TABLE IF NOT EXISTS user_cameras (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    label       TEXT    NOT NULL,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS user_cameras_user_label_idx
    ON user_cameras(user_id, lower(trim(label)));

CREATE INDEX IF NOT EXISTS user_cameras_user_id_idx ON user_cameras(user_id);

CREATE TABLE IF NOT EXISTS user_lenses (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT    NOT NULL,
    focal_mm    REAL    NOT NULL,
    aperture    REAL    NOT NULL,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS user_lenses_user_name_idx
    ON user_lenses(user_id, name);

CREATE INDEX IF NOT EXISTS user_lenses_user_id_idx ON user_lenses(user_id);

ALTER TABLE tickets ADD COLUMN camera_id INTEGER REFERENCES user_cameras(id) ON DELETE SET NULL;
ALTER TABLE tickets ADD COLUMN lens_id INTEGER REFERENCES user_lenses(id) ON DELETE SET NULL;
ALTER TABLE tickets ADD COLUMN film_iso INTEGER;

ALTER TABLE analog_ingest_jobs ADD COLUMN camera_id INTEGER REFERENCES user_cameras(id) ON DELETE SET NULL;
ALTER TABLE analog_ingest_jobs ADD COLUMN lens_id INTEGER REFERENCES user_lenses(id) ON DELETE SET NULL;
ALTER TABLE analog_ingest_jobs ADD COLUMN film_iso INTEGER;
