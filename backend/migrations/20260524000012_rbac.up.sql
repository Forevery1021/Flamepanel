CREATE TABLE IF NOT EXISTS roles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    is_system INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS permissions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    resource TEXT NOT NULL,
    action TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS role_permissions (
    role_id INTEGER NOT NULL,
    permission_id INTEGER NOT NULL,
    PRIMARY KEY (role_id, permission_id),
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
    FOREIGN KEY (permission_id) REFERENCES permissions(id) ON DELETE CASCADE
);

INSERT INTO roles (name, description, is_system) VALUES ('admin', '系统管理员，拥有所有权限', 1);
INSERT INTO roles (name, description, is_system) VALUES ('operator', '运维操作员，可管理大部分资源', 1);
INSERT INTO roles (name, description, is_system) VALUES ('viewer', '只读用户，仅可查看', 1);

INSERT INTO permissions (name, resource, action, description) VALUES
('dashboard:view', 'dashboard', 'view', '查看仪表盘'),
('file:manage', 'file', 'manage', '文件管理'),
('docker:manage', 'docker', 'manage', 'Docker 容器管理'),
('database:manage', 'database', 'manage', '数据库管理'),
('website:manage', 'website', 'manage', '网站管理'),
('waf:manage', 'waf', 'manage', 'WAF 防火墙管理'),
('terminal:access', 'terminal', 'access', 'Web 终端访问'),
('users:manage', 'users', 'manage', '用户与角色管理'),
('logs:view', 'logs', 'view', '操作日志查看'),
('process:view', 'process', 'view', '进程查看与管理'),
('system:cleanup', 'system', 'cleanup', '系统清理'),
('settings:manage', 'settings', 'manage', '面板设置'),
('cron:manage', 'cron', 'manage', '计划任务管理'),
('appstore:manage', 'appstore', 'manage', '应用商店管理'),
('ai:access', 'ai', 'access', 'AI 助手访问'),
('nodes:manage', 'nodes', 'manage', '节点管理'),
('backup:manage', 'backup', 'manage', '备份管理'),
('alerts:manage', 'alerts', 'manage', '告警管理'),
('plugins:manage', 'plugins', 'manage', '插件扩展管理');

INSERT INTO role_permissions (role_id, permission_id)
SELECT 1, id FROM permissions;

INSERT INTO role_permissions (role_id, permission_id)
SELECT 2, id FROM permissions WHERE name IN (
    'dashboard:view', 'file:manage', 'docker:manage', 'database:manage',
    'website:manage', 'waf:manage', 'terminal:access', 'logs:view',
    'process:view', 'system:cleanup', 'cron:manage', 'appstore:manage',
    'ai:access', 'nodes:manage', 'backup:manage', 'alerts:manage', 'plugins:manage'
);

INSERT INTO role_permissions (role_id, permission_id)
SELECT 3, id FROM permissions WHERE name IN (
    'dashboard:view', 'logs:view', 'process:view'
);
