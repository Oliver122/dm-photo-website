CREATE TABLE IF NOT EXISTS tickets (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id            INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    order_number       TEXT    NOT NULL,
    customer_no        TEXT,
    shop_no            TEXT,
    order_no           TEXT,
    summary_state_code TEXT    NOT NULL,
    summary_state_text TEXT,
    status             TEXT    NOT NULL DEFAULT 'open',
    created_at         TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS tickets_user_id_idx ON tickets(user_id);
CREATE INDEX IF NOT EXISTS tickets_order_number_idx ON tickets(order_number);
