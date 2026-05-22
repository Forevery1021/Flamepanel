-- migrations/0001_create_users.sql
--
-- users 表：存储管理员账号
-- password_hash 存 bcrypt 哈希（$2b$ 前缀）
--
-- 初始管理员账号通过环境变量 OP_ADMIN_USERNAME / OP_ADMIN_PASSWORD 在首次启动时写入
-- （见 application.rs 的 seed_admin 函数）

CREATE TABLE IF NOT EXISTS users (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    username      TEXT    NOT NULL UNIQUE,
    password_hash TEXT    NOT NULL,
    created_at    TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- 触发器：自动更新 updated_at
CREATE TRIGGER IF NOT EXISTS users_updated_at
AFTER UPDATE ON users
BEGIN
    UPDATE users SET updated_at = datetime('now') WHERE id = NEW.id;
END;