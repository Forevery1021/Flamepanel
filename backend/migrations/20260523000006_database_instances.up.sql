CREATE TABLE database_instances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    db_type TEXT NOT NULL CHECK(db_type IN ('mysql','mariadb','postgresql','redis','mongodb')),
    version TEXT NOT NULL DEFAULT 'latest',
    port INTEGER NOT NULL,
    container_id TEXT,
    username TEXT NOT NULL DEFAULT 'root',
    password TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'stopped' CHECK(status IN ('running','stopped','error','installing')),
    data_dir TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TRIGGER database_instances_updated_at AFTER UPDATE ON database_instances
BEGIN
    UPDATE database_instances SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TABLE database_backups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    instance_id INTEGER NOT NULL,
    filename TEXT NOT NULL,
    size_bytes INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (instance_id) REFERENCES database_instances(id) ON DELETE CASCADE
);
