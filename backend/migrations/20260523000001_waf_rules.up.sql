-- WAF 规则表
CREATE TABLE IF NOT EXISTS waf_rules (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL,
    pattern     TEXT    NOT NULL,
    target      TEXT    NOT NULL DEFAULT 'url',
    action      TEXT    NOT NULL DEFAULT 'block',
    description TEXT,
    enabled     INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TRIGGER IF NOT EXISTS waf_rules_updated_at
AFTER UPDATE ON waf_rules
BEGIN
    UPDATE waf_rules SET updated_at = datetime('now') WHERE id = NEW.id;
END;

-- 插入默认 WAF 规则
INSERT INTO waf_rules (name, pattern, target, action, description) VALUES
    ('Block SQL Injection', '(?i)(union\s+select|select.*from|insert\s+into|drop\s+table|--|;--)', 'url', 'block', '拦截常见 SQL 注入攻击'),
    ('Block XSS', '(?i)(<script>|javascript:|onerror=|onload=|<iframe)', 'url', 'block', '拦截跨站脚本攻击'),
    ('Block Path Traversal', '(\.\.\/|\.\.\\)', 'url', 'block', '拦截目录遍历攻击'),
    ('Log Suspicious User-Agent', '(?i)(nikto|sqlmap|burpsuite|nmap)', 'header', 'log', '记录可疑扫描工具'),
    ('Rate Limit POST', '.*', 'url', 'log', '记录所有 POST 请求（用于速率限制）');
