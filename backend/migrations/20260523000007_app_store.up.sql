CREATE TABLE installed_apps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    app_key TEXT NOT NULL,
    name TEXT NOT NULL,
    category TEXT NOT NULL,
    port INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'stopped' CHECK(status IN ('running','stopped','error','installing')),
    compose_file TEXT,
    data_dir TEXT,
    version TEXT NOT NULL DEFAULT 'latest',
    description TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TRIGGER installed_apps_updated_at AFTER UPDATE ON installed_apps
BEGIN
    UPDATE installed_apps SET updated_at = datetime('now') WHERE id = NEW.id;
END;
