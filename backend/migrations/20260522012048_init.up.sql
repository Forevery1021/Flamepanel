-- users 表：管理员/用户账号
CREATE TABLE IF NOT EXISTS users (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    username      TEXT    NOT NULL UNIQUE,
    password_hash TEXT    NOT NULL,
    role          TEXT    NOT NULL DEFAULT 'user',
    created_at    TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT    NOT NULL DEFAULT (datetime('now')),
    last_login    TEXT
);

-- users updated_at 触发器
CREATE TRIGGER IF NOT EXISTS users_updated_at
AFTER UPDATE ON users
BEGIN
    UPDATE users SET updated_at = datetime('now') WHERE id = NEW.id;
END;

-- websites 表：Nginx 站点管理
CREATE TABLE IF NOT EXISTS websites (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    domain        TEXT    NOT NULL UNIQUE,
    root_path     TEXT    NOT NULL,
    proxy_port    INTEGER,
    ssl_enabled   INTEGER NOT NULL DEFAULT 0,
    ssl_cert_path TEXT,
    ssl_key_path  TEXT,
    config_path   TEXT    NOT NULL,
    enabled       INTEGER NOT NULL DEFAULT 1,
    created_at    TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- websites updated_at 触发器
CREATE TRIGGER IF NOT EXISTS websites_updated_at
AFTER UPDATE ON websites
BEGIN
    UPDATE websites SET updated_at = datetime('now') WHERE id = NEW.id;
END;

-- operation_logs 表：操作审计日志
CREATE TABLE IF NOT EXISTS operation_logs (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    username   TEXT    NOT NULL,
    action     TEXT    NOT NULL,
    target     TEXT,
    ip         TEXT,
    created_at TEXT    NOT NULL DEFAULT (datetime('now'))
);
