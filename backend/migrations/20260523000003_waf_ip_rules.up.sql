CREATE TABLE IF NOT EXISTS waf_ip_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ip TEXT NOT NULL,
    action TEXT NOT NULL DEFAULT 'block' CHECK(action IN ('allow', 'block')),
    description TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TRIGGER IF NOT EXISTS waf_ip_rules_updated_at
    AFTER UPDATE ON waf_ip_rules
BEGIN
    UPDATE waf_ip_rules SET updated_at = datetime('now') WHERE id = NEW.id;
END;
