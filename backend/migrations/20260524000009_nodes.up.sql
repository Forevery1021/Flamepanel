CREATE TABLE IF NOT EXISTS nodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    host TEXT NOT NULL,
    agent_port INTEGER NOT NULL DEFAULT 9527,
    auth_token TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'online',
    cpu_usage REAL NOT NULL DEFAULT 0,
    memory_usage_percent REAL NOT NULL DEFAULT 0,
    disk_usage_percent REAL NOT NULL DEFAULT 0,
    load_one REAL NOT NULL DEFAULT 0,
    last_heartbeat TEXT NOT NULL DEFAULT (datetime('now')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
