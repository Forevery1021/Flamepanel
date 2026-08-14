pub struct PermissionRule {
    pub method: &'static str,
    pub exact: Option<&'static str>,
    pub prefix: Option<&'static str>,
    pub suffix: Option<&'static str>,
    pub contains: Option<&'static str>,
    pub not_contains: &'static [&'static str],
    pub not_suffix: &'static [&'static str],
    pub resource: &'static str,
    pub action: &'static str,
}

impl PermissionRule {
    pub const fn exact(
        method: &'static str,
        path: &'static str,
        resource: &'static str,
        action: &'static str,
    ) -> Self {
        Self {
            method,
            exact: Some(path),
            prefix: None,
            suffix: None,
            contains: None,
            not_contains: &[],
            not_suffix: &[],
            resource,
            action,
        }
    }

    pub const fn prefix(
        method: &'static str,
        prefix: &'static str,
        resource: &'static str,
        action: &'static str,
    ) -> Self {
        Self {
            method,
            exact: None,
            prefix: Some(prefix),
            suffix: None,
            contains: None,
            not_contains: &[],
            not_suffix: &[],
            resource,
            action,
        }
    }

    pub const fn suffix(
        method: &'static str,
        suffix: &'static str,
        resource: &'static str,
        action: &'static str,
    ) -> Self {
        Self {
            method,
            exact: None,
            prefix: None,
            suffix: Some(suffix),
            contains: None,
            not_contains: &[],
            not_suffix: &[],
            resource,
            action,
        }
    }

    pub const fn pre_suf(
        method: &'static str,
        prefix: &'static str,
        suffix: &'static str,
        resource: &'static str,
        action: &'static str,
    ) -> Self {
        Self {
            method,
            exact: None,
            prefix: Some(prefix),
            suffix: Some(suffix),
            contains: None,
            not_contains: &[],
            not_suffix: &[],
            resource,
            action,
        }
    }

    pub const fn pre_contains(
        method: &'static str,
        prefix: &'static str,
        contains: &'static str,
        resource: &'static str,
        action: &'static str,
    ) -> Self {
        Self {
            method,
            exact: None,
            prefix: Some(prefix),
            suffix: None,
            contains: Some(contains),
            not_contains: &[],
            not_suffix: &[],
            resource,
            action,
        }
    }

    pub const fn pre_not_suffix(
        method: &'static str,
        prefix: &'static str,
        not_suffix: &'static [&'static str],
        resource: &'static str,
        action: &'static str,
    ) -> Self {
        Self {
            method,
            exact: None,
            prefix: Some(prefix),
            suffix: None,
            contains: None,
            not_contains: &[],
            not_suffix,
            resource,
            action,
        }
    }

    /// 前缀 + 非子串包含（还原原 match 中 `starts_with && !contains` 组合）。
    pub const fn pre_not_contains(
        method: &'static str,
        prefix: &'static str,
        not_contains: &'static [&'static str],
        resource: &'static str,
        action: &'static str,
    ) -> Self {
        Self {
            method,
            exact: None,
            prefix: Some(prefix),
            suffix: None,
            contains: None,
            not_contains,
            not_suffix: &[],
            resource,
            action,
        }
    }

    /// 前缀 + 非子串包含 + 非后缀（还原原 match 中 `starts_with && !ends_with && !contains` 组合）。
    pub const fn pre_has_not_suf(
        method: &'static str,
        prefix: &'static str,
        not_suffix: &'static [&'static str],
        not_contains: &'static [&'static str],
        resource: &'static str,
        action: &'static str,
    ) -> Self {
        Self {
            method,
            exact: None,
            prefix: Some(prefix),
            suffix: None,
            contains: None,
            not_contains,
            not_suffix,
            resource,
            action,
        }
    }

    pub const fn contains(
        method: &'static str,
        contains: &'static str,
        resource: &'static str,
        action: &'static str,
    ) -> Self {
        Self {
            method,
            exact: None,
            prefix: None,
            suffix: None,
            contains: Some(contains),
            not_contains: &[],
            not_suffix: &[],
            resource,
            action,
        }
    }

    pub const fn contains_not(
        method: &'static str,
        contains: &'static str,
        not_contains: &'static [&'static str],
        resource: &'static str,
        action: &'static str,
    ) -> Self {
        Self {
            method,
            exact: None,
            prefix: None,
            suffix: None,
            contains: Some(contains),
            not_contains,
            not_suffix: &[],
            resource,
            action,
        }
    }

    fn matches(&self, method: &str, path: &str) -> bool {
        if method != self.method {
            return false;
        }
        if let Some(exact) = self.exact {
            return path == exact;
        }
        if let Some(prefix) = self.prefix {
            if !path.starts_with(prefix) {
                return false;
            }
        }
        if let Some(suffix) = self.suffix {
            if !path.ends_with(suffix) {
                return false;
            }
        }
        if let Some(needle) = self.contains {
            if !path.contains(needle) {
                return false;
            }
        }
        if self.not_contains.iter().any(|f| path.contains(f)) {
            return false;
        }
        if self.not_suffix.iter().any(|f| path.ends_with(f)) {
            return false;
        }
        true
    }
}

/// 声明式权限映射表（顺序敏感，首个命中生效，与历史 match 语义一致）。
pub static ROUTE_PERMISSIONS: &[PermissionRule] = &[
    // auth / users
    PermissionRule::exact("POST", "/api/auth/rotate-secret", "user", "update"),
    PermissionRule::exact("GET", "/api/users", "user", "read"),
    PermissionRule::exact("POST", "/api/users", "user", "create"),
    PermissionRule::prefix("PUT", "/api/users/", "user", "update"),
    PermissionRule::prefix("DELETE", "/api/users/", "user", "delete"),
    // nodes
    PermissionRule::exact("GET", "/api/nodes", "node", "read"),
    PermissionRule::exact("POST", "/api/nodes", "node", "create"),
    PermissionRule::exact("POST", "/api/nodes/register", "node", "create"),
    PermissionRule::exact("POST", "/api/nodes/batch-execute", "node", "execute"),
    PermissionRule::pre_suf("POST", "/api/nodes/", "/execute", "node", "execute"),
    PermissionRule::pre_suf("POST", "/api/nodes/", "/files/upload", "node", "execute"),
    PermissionRule::pre_contains("GET", "/api/nodes/", "/files", "node", "execute"),
    PermissionRule::prefix("GET", "/api/nodes/", "node", "read"),
    PermissionRule::prefix("PUT", "/api/nodes/", "node", "update"),
    PermissionRule::prefix("DELETE", "/api/nodes/", "node", "delete"),
    // websites
    PermissionRule::exact("GET", "/api/websites", "website", "read"),
    PermissionRule::exact("POST", "/api/websites", "website", "create"),
    PermissionRule::prefix("GET", "/api/websites/", "website", "read"),
    PermissionRule::prefix("PUT", "/api/websites/", "website", "update"),
    PermissionRule::prefix("DELETE", "/api/websites/", "website", "delete"),
    // docker: containers
    PermissionRule::exact("GET", "/api/docker/containers", "docker", "read"),
    PermissionRule::pre_suf("GET", "/api/docker/containers/", "/logs", "docker", "read"),
    PermissionRule::pre_suf("GET", "/api/docker/containers/", "/stats", "docker", "read"),
    PermissionRule::pre_suf(
        "GET",
        "/api/docker/containers/",
        "/inspect",
        "docker",
        "read",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/docker/containers/",
        "/start",
        "docker",
        "start",
    ),
    PermissionRule::pre_suf("POST", "/api/docker/containers/", "/stop", "docker", "stop"),
    PermissionRule::pre_suf(
        "POST",
        "/api/docker/containers/",
        "/restart",
        "docker",
        "start",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/docker/containers/",
        "/remove",
        "docker",
        "delete",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/docker/containers/",
        "/rename",
        "docker",
        "update",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/docker/containers/",
        "/pause",
        "docker",
        "start",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/docker/containers/",
        "/unpause",
        "docker",
        "start",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/docker/containers/",
        "/kill",
        "docker",
        "start",
    ),
    PermissionRule::exact("POST", "/api/docker/containers/prune", "docker", "delete"),
    // docker: images
    PermissionRule::exact("GET", "/api/docker/images", "docker", "read"),
    PermissionRule::exact("POST", "/api/docker/images/pull", "docker", "start"),
    PermissionRule::pre_suf("POST", "/api/docker/images/", "/remove", "docker", "delete"),
    PermissionRule::pre_suf("POST", "/api/docker/images/", "/tag", "docker", "update"),
    PermissionRule::exact("POST", "/api/docker/images/prune", "docker", "delete"),
    // docker: networks
    PermissionRule::exact("GET", "/api/docker/networks", "docker", "read"),
    PermissionRule::exact("POST", "/api/docker/networks", "docker", "create"),
    PermissionRule::pre_not_suffix(
        "DELETE",
        "/api/docker/networks/",
        &["/prune"],
        "docker",
        "delete",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/docker/networks/",
        "/connect",
        "docker",
        "update",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/docker/networks/",
        "/disconnect",
        "docker",
        "update",
    ),
    PermissionRule::exact("POST", "/api/docker/networks/prune", "docker", "delete"),
    // docker: volumes
    PermissionRule::exact("GET", "/api/docker/volumes", "docker", "read"),
    PermissionRule::exact("POST", "/api/docker/volumes", "docker", "create"),
    PermissionRule::pre_not_suffix(
        "DELETE",
        "/api/docker/volumes/",
        &["/prune"],
        "docker",
        "delete",
    ),
    PermissionRule::exact("POST", "/api/docker/volumes/prune", "docker", "delete"),
    // docker: compose
    PermissionRule::exact("GET", "/api/docker/compose", "docker", "read"),
    PermissionRule::exact("POST", "/api/docker/compose/deploy", "docker", "start"),
    PermissionRule::pre_suf("POST", "/api/docker/compose/", "/up", "docker", "start"),
    PermissionRule::pre_suf("POST", "/api/docker/compose/", "/down", "docker", "stop"),
    // plugins
    PermissionRule::exact("GET", "/api/plugins", "plugin", "read"),
    PermissionRule::exact("POST", "/api/plugins", "plugin", "create"),
    PermissionRule::pre_not_contains(
        "GET",
        "/api/plugins/",
        &[
            "/execute/",
            "/enable",
            "/disable",
            "/settings",
            "/metrics",
            "/reload",
        ],
        "plugin",
        "read",
    ),
    PermissionRule::pre_suf("POST", "/api/plugins/", "/enable", "plugin", "create"),
    PermissionRule::pre_suf("POST", "/api/plugins/", "/disable", "plugin", "create"),
    PermissionRule::pre_contains("POST", "/api/plugins/", "/execute/", "plugin", "execute"),
    PermissionRule::pre_has_not_suf(
        "POST",
        "/api/plugins/",
        &["/enable", "/disable"],
        &["/execute/", "/reload", "/settings"],
        "plugin",
        "delete",
    ),
    PermissionRule::pre_contains("POST", "/api/plugins/", "/reload", "plugin", "create"),
    PermissionRule::pre_contains("GET", "/api/plugins/", "/settings", "plugin", "config"),
    PermissionRule::pre_contains("POST", "/api/plugins/", "/settings", "plugin", "config"),
    PermissionRule::pre_contains("GET", "/api/plugins/", "/metrics", "plugin", "read"),
    PermissionRule::pre_contains("DELETE", "/api/plugins/", "/metrics", "plugin", "config"),
    // web-servers
    PermissionRule::exact("GET", "/api/web-servers/engines", "web_server", "read"),
    PermissionRule::exact("GET", "/api/web-servers", "web_server", "read"),
    PermissionRule::exact("POST", "/api/web-servers", "web_server", "create"),
    PermissionRule::pre_not_contains(
        "GET",
        "/api/web-servers/",
        &[
            "/start",
            "/stop",
            "/restart",
            "/reload",
            "/configtest",
            "/config",
        ],
        "web_server",
        "read",
    ),
    PermissionRule::pre_suf("POST", "/api/web-servers/", "/start", "web_server", "start"),
    PermissionRule::pre_suf("POST", "/api/web-servers/", "/stop", "web_server", "stop"),
    PermissionRule::pre_suf(
        "POST",
        "/api/web-servers/",
        "/restart",
        "web_server",
        "start",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/web-servers/",
        "/reload",
        "web_server",
        "reload",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/web-servers/",
        "/configtest",
        "web_server",
        "configtest",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/web-servers/",
        "/switch-engine",
        "web_server",
        "update",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/web-servers/",
        "/preset",
        "web_server",
        "update",
    ),
    PermissionRule::exact(
        "GET",
        "/api/web-servers/native/detect",
        "web_server",
        "read",
    ),
    PermissionRule::exact(
        "POST",
        "/api/web-servers/native/install",
        "web_server",
        "create",
    ),
    PermissionRule::exact(
        "POST",
        "/api/web-servers/native/uninstall",
        "web_server",
        "delete",
    ),
    PermissionRule::exact(
        "POST",
        "/api/web-servers/native/autostart",
        "web_server",
        "update",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/web-servers/",
        "/autostart",
        "web_server",
        "update",
    ),
    PermissionRule::pre_suf(
        "GET",
        "/api/web-servers/",
        "/native-status",
        "web_server",
        "read",
    ),
    PermissionRule::exact("GET", "/api/web-servers/presets", "web_server", "read"),
    PermissionRule::pre_suf(
        "POST",
        "/api/websites/",
        "/switch-engine",
        "website",
        "update",
    ),
    PermissionRule::pre_contains("GET", "/api/web-servers/", "/config", "web_server", "read"),
    PermissionRule::pre_not_contains(
        "PUT",
        "/api/web-servers/",
        &[
            "/start",
            "/stop",
            "/restart",
            "/reload",
            "/configtest",
            "/config",
        ],
        "web_server",
        "update",
    ),
    PermissionRule::pre_not_contains(
        "DELETE",
        "/api/web-servers/",
        &[
            "/start",
            "/stop",
            "/restart",
            "/reload",
            "/configtest",
            "/config",
        ],
        "web_server",
        "delete",
    ),
    // settings
    PermissionRule::exact("GET", "/api/settings", "settings", "read"),
    PermissionRule::prefix("GET", "/api/settings/", "settings", "read"),
    PermissionRule::exact("PUT", "/api/settings", "settings", "update"),
    // app-store（置于 database 全局后缀规则之前，避免 install/uninstall 被误映射为 database 权限）
    PermissionRule::exact("GET", "/api/app-store/packages", "app_store", "read"),
    PermissionRule::exact("GET", "/api/app-store/wasm-builtins", "app_store", "read"),
    PermissionRule::prefix("GET", "/api/app-store/packages/", "app_store", "read"),
    PermissionRule::pre_suf(
        "POST",
        "/api/app-store/packages/",
        "/install",
        "app_store",
        "create",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/app-store/packages/",
        "/import",
        "app_store",
        "create",
    ),
    PermissionRule::exact("GET", "/api/app-store/installed", "app_store", "read"),
    PermissionRule::pre_not_suffix(
        "GET",
        "/api/app-store/installed/",
        &["/upgrade", "/uninstall", "/logs"],
        "app_store",
        "read",
    ),
    PermissionRule::pre_suf(
        "GET",
        "/api/app-store/installed/",
        "/logs",
        "app_store",
        "read",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/app-store/installed/",
        "/upgrade",
        "app_store",
        "update",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/app-store/installed/",
        "/launch",
        "app_store",
        "read",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/app-store/installed/",
        "/uninstall",
        "app_store",
        "delete",
    ),
    // databases（含历史全局后缀/包含规则，顺序敏感，勿随意调整）
    PermissionRule::exact("GET", "/api/databases", "database", "read"),
    PermissionRule::pre_not_contains(
        "GET",
        "/api/databases/",
        &[
            "/start",
            "/stop",
            "/restart",
            "/status",
            "/uninstall",
            "/databases/",
            "/users",
        ],
        "database",
        "read",
    ),
    PermissionRule::pre_not_contains(
        "DELETE",
        "/api/databases/",
        &["/uninstall"],
        "database",
        "delete",
    ),
    PermissionRule::suffix("POST", "/uninstall", "database", "delete"),
    PermissionRule::suffix("POST", "/install", "database", "create"),
    PermissionRule::suffix("POST", "/start", "database", "start"),
    PermissionRule::suffix("POST", "/stop", "database", "stop"),
    PermissionRule::suffix("POST", "/restart", "database", "start"),
    PermissionRule::contains("GET", "/status", "database", "read"),
    PermissionRule::contains_not("POST", "/databases", &["/delete"], "database", "create"),
    PermissionRule::contains("DELETE", "/databases/", "database", "delete"),
    PermissionRule::contains("POST", "/users", "database", "update"),
    PermissionRule::contains("DELETE", "/users/", "database", "update"),
    // files
    PermissionRule::exact("GET", "/api/files", "file", "read"),
    PermissionRule::exact("GET", "/api/files/read", "file", "read"),
    PermissionRule::exact("GET", "/api/files/download", "file", "upload"),
    PermissionRule::exact("POST", "/api/files/write", "file", "write"),
    PermissionRule::exact("POST", "/api/files/create-file", "file", "write"),
    PermissionRule::exact("POST", "/api/files/create-dir", "file", "write"),
    PermissionRule::exact("DELETE", "/api/files/delete", "file", "write"),
    PermissionRule::exact("POST", "/api/files/rename", "file", "write"),
    PermissionRule::exact("POST", "/api/files/chmod", "file", "write"),
    PermissionRule::exact("POST", "/api/files/upload", "file", "upload"),
    // scheduled tasks（置于 firewall 之前，避免 /toggle 被 firewall 全局后缀规则抢先映射）
    PermissionRule::exact("GET", "/api/scheduled-tasks", "scheduled_task", "read"),
    PermissionRule::exact("POST", "/api/scheduled-tasks", "scheduled_task", "create"),
    PermissionRule::pre_not_suffix(
        "GET",
        "/api/scheduled-tasks/",
        &["/run"],
        "scheduled_task",
        "read",
    ),
    PermissionRule::prefix("PUT", "/api/scheduled-tasks/", "scheduled_task", "update"),
    PermissionRule::prefix(
        "DELETE",
        "/api/scheduled-tasks/",
        "scheduled_task",
        "delete",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/scheduled-tasks/",
        "/run",
        "scheduled_task",
        "execute",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/scheduled-tasks/",
        "/toggle",
        "scheduled_task",
        "update",
    ),
    // firewall
    PermissionRule::exact("GET", "/api/firewall/rules", "firewall", "read"),
    PermissionRule::pre_not_suffix(
        "GET",
        "/api/firewall/rules/",
        &["/toggle"],
        "firewall",
        "read",
    ),
    PermissionRule::exact("POST", "/api/firewall/rules", "firewall", "create"),
    PermissionRule::prefix("PUT", "/api/firewall/rules/", "firewall", "update"),
    PermissionRule::prefix("DELETE", "/api/firewall/rules/", "firewall", "delete"),
    PermissionRule::suffix("POST", "/toggle", "firewall", "enable"),
    PermissionRule::exact("POST", "/api/firewall/apply", "firewall", "apply"),
    PermissionRule::exact("GET", "/api/firewall/status", "firewall", "read"),
    PermissionRule::exact("POST", "/api/firewall/enable", "firewall", "enable"),
    PermissionRule::exact("POST", "/api/firewall/disable", "firewall", "enable"),
    PermissionRule::exact("POST", "/api/firewall/reorder", "firewall", "update"),
    // memos
    PermissionRule::exact("GET", "/api/memos", "memo", "read"),
    PermissionRule::exact("POST", "/api/memos", "memo", "create"),
    PermissionRule::prefix("PUT", "/api/memos/", "memo", "update"),
    PermissionRule::prefix("DELETE", "/api/memos/", "memo", "delete"),
    // operation logs / logs
    PermissionRule::exact("GET", "/api/operation-logs", "operation_log", "read"),
    PermissionRule::exact("GET", "/api/operation-logs/export", "operation_log", "read"),
    PermissionRule::prefix("DELETE", "/api/operation-logs/", "operation_log", "delete"),
    // outbox（事件落库）
    PermissionRule::exact("GET", "/api/outbox-events", "outbox", "read"),
    PermissionRule::exact("GET", "/api/logs", "log", "read"),
    PermissionRule::prefix("DELETE", "/api/logs/", "log", "delete"),
    // backups
    PermissionRule::exact("GET", "/api/backups", "backup", "read"),
    PermissionRule::exact("POST", "/api/backups", "backup", "create"),
    PermissionRule::prefix("GET", "/api/backups/", "backup", "read"),
    PermissionRule::prefix("DELETE", "/api/backups/", "backup", "delete"),
    PermissionRule::prefix("POST", "/api/backups/", "backup", "create"),
    // metrics
    PermissionRule::exact("GET", "/api/metrics/processes", "node", "read"),
    // nodes remote action（节点远程动作）
    PermissionRule::pre_suf("POST", "/api/nodes/", "/action", "node", "execute"),
    // docker 容器按 id 读取（get container）
    PermissionRule::prefix("GET", "/api/docker/containers/", "docker", "read"),
    // app-store 批量导入
    PermissionRule::exact(
        "POST",
        "/api/app-store/packages/batch-import",
        "app_store",
        "create",
    ),
    // databases 批量状态更新
    PermissionRule::exact("PATCH", "/api/databases/batch-status", "database", "update"),
    // settings 批量更新
    PermissionRule::exact("PATCH", "/api/settings/batch", "settings", "update"),
    // tasks（统一任务查询/取消/清理，Stage 5 补齐声明）
    PermissionRule::exact("GET", "/api/tasks", "task", "read"),
    PermissionRule::prefix("GET", "/api/tasks/", "task", "read"),
    PermissionRule::exact("POST", "/api/tasks/prune", "task", "delete"),
    PermissionRule::pre_suf("POST", "/api/tasks/", "/cancel", "task", "execute"),
];

/// 查询请求对应的资源/动作权限（未登记返回 None，表示不额外校验 RBAC）。
pub fn route_permission(
    method: &axum::http::Method,
    path: &str,
) -> Option<(&'static str, &'static str)> {
    let path = path.trim_end_matches('/');
    let method = method.as_str();
    ROUTE_PERMISSIONS
        .iter()
        .find(|r| r.matches(method, path))
        .map(|r| (r.resource, r.action))
}

// ── 权限表测试（Stage3.4）────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Method;

    fn perm(method: &str, path: &str) -> Option<(&'static str, &'static str)> {
        route_permission(&Method::from_bytes(method.as_bytes()).unwrap(), path)
    }

    #[test]
    fn permission_crud_paths_map_correctly() {
        // users
        assert_eq!(perm("GET", "/api/users"), Some(("user", "read")));
        assert_eq!(perm("POST", "/api/users"), Some(("user", "create")));
        assert_eq!(perm("PUT", "/api/users/1"), Some(("user", "update")));
        assert_eq!(perm("DELETE", "/api/users/1"), Some(("user", "delete")));
        // nodes
        assert_eq!(perm("GET", "/api/nodes"), Some(("node", "read")));
        assert_eq!(perm("GET", "/api/nodes/1"), Some(("node", "read")));
        assert_eq!(perm("POST", "/api/nodes"), Some(("node", "create")));
        assert_eq!(perm("DELETE", "/api/nodes/1"), Some(("node", "delete")));
        // nodes stage5 remote ops
        assert_eq!(
            perm("POST", "/api/nodes/register"),
            Some(("node", "create"))
        );
        assert_eq!(
            perm("POST", "/api/nodes/1/execute"),
            Some(("node", "execute"))
        );
        assert_eq!(
            perm("POST", "/api/nodes/batch-execute"),
            Some(("node", "execute"))
        );
        assert_eq!(perm("GET", "/api/nodes/1/files"), Some(("node", "execute")));
        assert_eq!(
            perm("GET", "/api/nodes/1/files/download"),
            Some(("node", "execute"))
        );
        assert_eq!(
            perm("POST", "/api/nodes/1/files/upload"),
            Some(("node", "execute"))
        );
        // websites
        assert_eq!(perm("GET", "/api/websites"), Some(("website", "read")));
        assert_eq!(perm("POST", "/api/websites"), Some(("website", "create")));
        assert_eq!(perm("PUT", "/api/websites/2"), Some(("website", "update")));
        // memos
        assert_eq!(perm("GET", "/api/memos"), Some(("memo", "read")));
        assert_eq!(perm("PUT", "/api/memos/3"), Some(("memo", "update")));
        assert_eq!(perm("DELETE", "/api/memos/3"), Some(("memo", "delete")));
    }

    #[test]
    fn permission_docker_routes_map_correctly() {
        assert_eq!(
            perm("GET", "/api/docker/containers"),
            Some(("docker", "read"))
        );
        assert_eq!(
            perm("GET", "/api/docker/containers/abc/logs"),
            Some(("docker", "read"))
        );
        assert_eq!(
            perm("GET", "/api/docker/containers/abc/stats"),
            Some(("docker", "read"))
        );
        assert_eq!(
            perm("GET", "/api/docker/containers/abc/inspect"),
            Some(("docker", "read"))
        );
        assert_eq!(
            perm("POST", "/api/docker/containers/abc/start"),
            Some(("docker", "start"))
        );
        assert_eq!(
            perm("POST", "/api/docker/containers/abc/stop"),
            Some(("docker", "stop"))
        );
        assert_eq!(
            perm("POST", "/api/docker/containers/abc/restart"),
            Some(("docker", "start"))
        );
        assert_eq!(
            perm("POST", "/api/docker/containers/abc/remove"),
            Some(("docker", "delete"))
        );
        assert_eq!(
            perm("POST", "/api/docker/containers/abc/rename"),
            Some(("docker", "update"))
        );
        assert_eq!(
            perm("POST", "/api/docker/containers/abc/pause"),
            Some(("docker", "start"))
        );
        assert_eq!(
            perm("POST", "/api/docker/containers/abc/kill"),
            Some(("docker", "start"))
        );
        assert_eq!(
            perm("POST", "/api/docker/containers/prune"),
            Some(("docker", "delete"))
        );
        // images
        assert_eq!(perm("GET", "/api/docker/images"), Some(("docker", "read")));
        assert_eq!(
            perm("POST", "/api/docker/images/pull"),
            Some(("docker", "start"))
        );
        assert_eq!(
            perm("POST", "/api/docker/images/xyz/remove"),
            Some(("docker", "delete"))
        );
        assert_eq!(
            perm("POST", "/api/docker/images/xyz/tag"),
            Some(("docker", "update"))
        );
        assert_eq!(
            perm("POST", "/api/docker/images/prune"),
            Some(("docker", "delete"))
        );
        // networks：prune 走精确规则，不匹配 delete 前缀规则
        assert_eq!(
            perm("GET", "/api/docker/networks"),
            Some(("docker", "read"))
        );
        assert_eq!(
            perm("POST", "/api/docker/networks"),
            Some(("docker", "create"))
        );
        assert_eq!(
            perm("DELETE", "/api/docker/networks/net1"),
            Some(("docker", "delete"))
        );
        assert_eq!(
            perm("POST", "/api/docker/networks/net1/connect"),
            Some(("docker", "update"))
        );
        assert_eq!(
            perm("POST", "/api/docker/networks/prune"),
            Some(("docker", "delete"))
        );
        // volumes
        assert_eq!(perm("GET", "/api/docker/volumes"), Some(("docker", "read")));
        assert_eq!(
            perm("POST", "/api/docker/volumes"),
            Some(("docker", "create"))
        );
        assert_eq!(
            perm("DELETE", "/api/docker/volumes/vol1"),
            Some(("docker", "delete"))
        );
        assert_eq!(
            perm("POST", "/api/docker/volumes/prune"),
            Some(("docker", "delete"))
        );
        // compose
        assert_eq!(perm("GET", "/api/docker/compose"), Some(("docker", "read")));
        assert_eq!(
            perm("POST", "/api/docker/compose/deploy"),
            Some(("docker", "start"))
        );
        assert_eq!(
            perm("POST", "/api/docker/compose/proj/up"),
            Some(("docker", "start"))
        );
        assert_eq!(
            perm("POST", "/api/docker/compose/proj/down"),
            Some(("docker", "stop"))
        );
    }

    #[test]
    fn permission_plugin_routes_map_correctly() {
        assert_eq!(perm("GET", "/api/plugins"), Some(("plugin", "read")));
        assert_eq!(perm("POST", "/api/plugins"), Some(("plugin", "create")));
        assert_eq!(perm("GET", "/api/plugins/p1"), Some(("plugin", "read")));
        assert_eq!(
            perm("POST", "/api/plugins/p1/enable"),
            Some(("plugin", "create"))
        );
        assert_eq!(
            perm("POST", "/api/plugins/p1/disable"),
            Some(("plugin", "create"))
        );
        assert_eq!(
            perm("POST", "/api/plugins/p1/execute/fn"),
            Some(("plugin", "execute"))
        );
        assert_eq!(
            perm("POST", "/api/plugins/p1/reload"),
            Some(("plugin", "create"))
        );
        assert_eq!(
            perm("POST", "/api/plugins/p1"), // unload = delete
            Some(("plugin", "delete"))
        );
        assert_eq!(
            perm("GET", "/api/plugins/p1/settings"),
            Some(("plugin", "config"))
        );
        assert_eq!(
            perm("GET", "/api/plugins/p1/settings/k"),
            Some(("plugin", "config"))
        );
        assert_eq!(
            perm("POST", "/api/plugins/p1/settings"),
            Some(("plugin", "config"))
        );
        assert_eq!(
            perm("GET", "/api/plugins/p1/metrics"),
            Some(("plugin", "read"))
        );
        assert_eq!(
            perm("DELETE", "/api/plugins/p1/metrics"),
            Some(("plugin", "config"))
        );
    }

    #[test]
    fn permission_web_server_routes_map_correctly() {
        assert_eq!(
            perm("GET", "/api/web-servers/engines"),
            Some(("web_server", "read"))
        );
        assert_eq!(
            perm("GET", "/api/web-servers"),
            Some(("web_server", "read"))
        );
        assert_eq!(
            perm("GET", "/api/web-servers/1"),
            Some(("web_server", "read"))
        );
        assert_eq!(
            perm("POST", "/api/web-servers"),
            Some(("web_server", "create"))
        );
        assert_eq!(
            perm("PUT", "/api/web-servers/1"),
            Some(("web_server", "update"))
        );
        assert_eq!(
            perm("DELETE", "/api/web-servers/1"),
            Some(("web_server", "delete"))
        );
        assert_eq!(
            perm("POST", "/api/web-servers/1/start"),
            Some(("web_server", "start"))
        );
        assert_eq!(
            perm("POST", "/api/web-servers/1/stop"),
            Some(("web_server", "stop"))
        );
        assert_eq!(
            perm("POST", "/api/web-servers/1/reload"),
            Some(("web_server", "reload"))
        );
        assert_eq!(
            perm("POST", "/api/web-servers/1/configtest"),
            Some(("web_server", "configtest"))
        );
        assert_eq!(
            perm("GET", "/api/web-servers/1/config"),
            Some(("web_server", "read"))
        );
        assert_eq!(
            perm("POST", "/api/web-servers/1/switch-engine"),
            Some(("web_server", "update"))
        );
        assert_eq!(
            perm("POST", "/api/web-servers/native/install"),
            Some(("web_server", "create"))
        );
        assert_eq!(
            perm("POST", "/api/web-servers/native/uninstall"),
            Some(("web_server", "delete"))
        );
        assert_eq!(perm("POST", "/api/web-servers/1/native-status"), None);
    }

    #[test]
    fn permission_settings_database_files_firewall() {
        assert_eq!(perm("GET", "/api/settings"), Some(("settings", "read")));
        assert_eq!(
            perm("GET", "/api/settings/smtp"),
            Some(("settings", "read"))
        );
        assert_eq!(perm("PUT", "/api/settings"), Some(("settings", "update")));
        assert_eq!(perm("GET", "/api/databases"), Some(("database", "read")));
        // 历史语义：数据库 read 规则排除含 /databases/ 子串的路径（子库/用户子路径不额外鉴权）
        assert_eq!(perm("GET", "/api/databases/1"), None);
        assert_eq!(
            perm("GET", "/api/databases/1/status"),
            Some(("database", "read"))
        );
        // 历史语义：子库列表路径被排除在 read 规则外（不额外鉴权），声明化后保持不变
        assert_eq!(perm("GET", "/api/databases/1/databases"), None);
        assert_eq!(
            perm("POST", "/api/databases/mysql/install"),
            Some(("database", "create"))
        );
        assert_eq!(
            perm("POST", "/api/databases/1/start"),
            Some(("database", "start"))
        );
        assert_eq!(
            perm("POST", "/api/databases/1/stop"),
            Some(("database", "stop"))
        );
        assert_eq!(
            perm("POST", "/api/databases/1/restart"),
            Some(("database", "start"))
        );
        assert_eq!(
            perm("DELETE", "/api/databases/1"),
            Some(("database", "delete"))
        );
        assert_eq!(
            perm("POST", "/api/databases/1/databases"),
            Some(("database", "create"))
        );
        assert_eq!(
            perm("DELETE", "/api/databases/1/databases/db1"),
            Some(("database", "delete"))
        );
        // 历史语义：POST /users 被 contains(/databases) 规则抢先匹配（database:create）
        assert_eq!(
            perm("POST", "/api/databases/1/users"),
            Some(("database", "create"))
        );
        // 历史语义：DELETE /users/... 被 contains(/databases/) 规则抢先匹配（database:delete）
        assert_eq!(
            perm("DELETE", "/api/databases/1/users/u1"),
            Some(("database", "delete"))
        );
        assert_eq!(
            perm("POST", "/api/databases/1/uninstall"),
            Some(("database", "delete"))
        );
        // files
        assert_eq!(perm("GET", "/api/files"), Some(("file", "read")));
        assert_eq!(perm("GET", "/api/files/read"), Some(("file", "read")));
        assert_eq!(perm("POST", "/api/files/write"), Some(("file", "write")));
        assert_eq!(perm("GET", "/api/files/download"), Some(("file", "upload")));
        assert_eq!(perm("POST", "/api/files/upload"), Some(("file", "upload")));
        // firewall
        assert_eq!(
            perm("GET", "/api/firewall/rules"),
            Some(("firewall", "read"))
        );
        assert_eq!(
            perm("GET", "/api/firewall/rules/1"),
            Some(("firewall", "read"))
        );
        assert_eq!(
            perm("POST", "/api/firewall/rules"),
            Some(("firewall", "create"))
        );
        assert_eq!(
            perm("PUT", "/api/firewall/rules/1"),
            Some(("firewall", "update"))
        );
        assert_eq!(
            perm("POST", "/api/firewall/rules/1/toggle"),
            Some(("firewall", "enable"))
        );
        assert_eq!(
            perm("POST", "/api/firewall/apply"),
            Some(("firewall", "apply"))
        );
        assert_eq!(
            perm("POST", "/api/firewall/enable"),
            Some(("firewall", "enable"))
        );
    }

    #[test]
    fn permission_app_store_backups_scheduled_tasks_logs() {
        assert_eq!(
            perm("GET", "/api/app-store/packages"),
            Some(("app_store", "read"))
        );
        assert_eq!(
            perm("GET", "/api/app-store/packages/nginx"),
            Some(("app_store", "read"))
        );
        assert_eq!(
            perm("POST", "/api/app-store/packages/nginx/install"),
            Some(("app_store", "create"))
        );
        assert_eq!(
            perm("POST", "/api/app-store/packages/nginx/import"),
            Some(("app_store", "create"))
        );
        assert_eq!(
            perm("GET", "/api/app-store/installed"),
            Some(("app_store", "read"))
        );
        assert_eq!(
            perm("GET", "/api/app-store/installed/1"),
            Some(("app_store", "read"))
        );
        assert_eq!(
            perm("GET", "/api/app-store/installed/1/logs"),
            Some(("app_store", "read"))
        );
        assert_eq!(
            perm("POST", "/api/app-store/installed/1/upgrade"),
            Some(("app_store", "update"))
        );
        assert_eq!(
            perm("POST", "/api/app-store/installed/1/launch"),
            Some(("app_store", "read"))
        );
        assert_eq!(
            perm("POST", "/api/app-store/installed/1/uninstall"),
            Some(("app_store", "delete"))
        );
        // backups
        assert_eq!(perm("GET", "/api/backups"), Some(("backup", "read")));
        assert_eq!(perm("POST", "/api/backups"), Some(("backup", "create")));
        assert_eq!(perm("GET", "/api/backups/app.db"), Some(("backup", "read")));
        assert_eq!(
            perm("DELETE", "/api/backups/app.db"),
            Some(("backup", "delete"))
        );
        assert_eq!(
            perm("POST", "/api/backups/app.db/restore"),
            Some(("backup", "create"))
        );
        // scheduled tasks
        assert_eq!(
            perm("GET", "/api/scheduled-tasks"),
            Some(("scheduled_task", "read"))
        );
        assert_eq!(
            perm("POST", "/api/scheduled-tasks"),
            Some(("scheduled_task", "create"))
        );
        assert_eq!(
            perm("GET", "/api/scheduled-tasks/1"),
            Some(("scheduled_task", "read"))
        );
        assert_eq!(
            perm("PUT", "/api/scheduled-tasks/1"),
            Some(("scheduled_task", "update"))
        );
        assert_eq!(
            perm("DELETE", "/api/scheduled-tasks/1"),
            Some(("scheduled_task", "delete"))
        );
        assert_eq!(
            perm("POST", "/api/scheduled-tasks/1/run"),
            Some(("scheduled_task", "execute"))
        );
        assert_eq!(
            perm("POST", "/api/scheduled-tasks/1/toggle"),
            Some(("scheduled_task", "update"))
        );
        // logs & operation-logs
        assert_eq!(perm("GET", "/api/logs"), Some(("log", "read")));
        assert_eq!(perm("DELETE", "/api/logs/1"), Some(("log", "delete")));
        assert_eq!(
            perm("GET", "/api/operation-logs"),
            Some(("operation_log", "read"))
        );
        assert_eq!(
            perm("DELETE", "/api/operation-logs/1"),
            Some(("operation_log", "delete"))
        );
    }

    #[test]
    fn permission_unregistered_paths_return_none() {
        assert_eq!(perm("GET", "/api/unknown"), None);
        assert_eq!(perm("GET", "/api/backups/1"), Some(("backup", "read")));
        assert_eq!(perm("PATCH", "/api/users/1"), None);
    }

    /// Stage 5（A2）路由↔权限一致性检查（Scheme B CI）。
    /// 枚举 `routes.rs` 中已注册的受保护路由（排除公开/WS/认证自服务），
    /// 断言每个路由都在 `ROUTE_PERMISSIONS` 中声明了权限，或在显式 auth-only 白名单中。
    /// 新增路由若忘记声明权限 → 本测试失败，从而"漏声明可被测试抓住"。
    #[test]
    fn permission_table_covers_all_routes() {
        // （method, path-template）——path 中的 {param} 以样例值 1 替换后匹配
        let routes: &[(&str, &str)] = &[
            // users
            ("GET", "/api/users"),
            ("POST", "/api/users"),
            ("PUT", "/api/users/1"),
            ("DELETE", "/api/users/1"),
            // auth（仅旋转密钥受 RBAC 保护；其余为认证自服务）
            ("POST", "/api/auth/rotate-secret"),
            // nodes
            ("GET", "/api/nodes"),
            ("POST", "/api/nodes"),
            ("POST", "/api/nodes/register"),
            ("PUT", "/api/nodes/1"),
            ("DELETE", "/api/nodes/1"),
            ("GET", "/api/nodes/1/status"),
            ("GET", "/api/nodes/1/metrics"),
            ("POST", "/api/nodes/1/execute"),
            ("POST", "/api/nodes/1/action"),
            ("POST", "/api/nodes/batch-execute"),
            ("GET", "/api/nodes/1/files"),
            ("GET", "/api/nodes/1/files/download"),
            ("POST", "/api/nodes/1/files/upload"),
            // websites
            ("GET", "/api/websites"),
            ("POST", "/api/websites"),
            ("GET", "/api/websites/1"),
            ("PUT", "/api/websites/1"),
            ("DELETE", "/api/websites/1"),
            ("POST", "/api/websites/1/switch-engine"),
            // docker
            ("GET", "/api/docker/containers"),
            ("GET", "/api/docker/containers/1"),
            ("GET", "/api/docker/containers/1/logs"),
            ("GET", "/api/docker/containers/1/stats"),
            ("GET", "/api/docker/containers/1/inspect"),
            ("POST", "/api/docker/containers/1/start"),
            ("POST", "/api/docker/containers/1/stop"),
            ("POST", "/api/docker/containers/1/restart"),
            ("POST", "/api/docker/containers/1/remove"),
            ("POST", "/api/docker/containers/1/rename"),
            ("POST", "/api/docker/containers/1/pause"),
            ("POST", "/api/docker/containers/1/unpause"),
            ("POST", "/api/docker/containers/1/kill"),
            ("POST", "/api/docker/containers/prune"),
            ("GET", "/api/docker/images"),
            ("POST", "/api/docker/images/pull"),
            ("POST", "/api/docker/images/1/remove"),
            ("POST", "/api/docker/images/1/tag"),
            ("POST", "/api/docker/images/prune"),
            ("GET", "/api/docker/networks"),
            ("POST", "/api/docker/networks"),
            ("DELETE", "/api/docker/networks/net1"),
            ("POST", "/api/docker/networks/net1/connect"),
            ("POST", "/api/docker/networks/net1/disconnect"),
            ("POST", "/api/docker/networks/prune"),
            ("GET", "/api/docker/volumes"),
            ("POST", "/api/docker/volumes"),
            ("DELETE", "/api/docker/volumes/vol1"),
            ("POST", "/api/docker/volumes/prune"),
            ("GET", "/api/docker/compose"),
            ("POST", "/api/docker/compose/deploy"),
            ("POST", "/api/docker/compose/proj/up"),
            ("POST", "/api/docker/compose/proj/down"),
            // plugins
            ("GET", "/api/plugins"),
            ("POST", "/api/plugins"),
            ("GET", "/api/plugins/p1"),
            ("POST", "/api/plugins/p1"),
            ("POST", "/api/plugins/p1/enable"),
            ("POST", "/api/plugins/p1/disable"),
            ("POST", "/api/plugins/p1/execute/fn"),
            ("POST", "/api/plugins/p1/reload"),
            ("GET", "/api/plugins/p1/settings"),
            ("POST", "/api/plugins/p1/settings"),
            ("GET", "/api/plugins/p1/settings/k"),
            ("GET", "/api/plugins/p1/metrics"),
            ("DELETE", "/api/plugins/p1/metrics"),
            // web-servers
            ("GET", "/api/web-servers/engines"),
            ("GET", "/api/web-servers"),
            ("POST", "/api/web-servers"),
            ("GET", "/api/web-servers/1"),
            ("PUT", "/api/web-servers/1"),
            ("DELETE", "/api/web-servers/1"),
            ("POST", "/api/web-servers/1/start"),
            ("POST", "/api/web-servers/1/stop"),
            ("POST", "/api/web-servers/1/restart"),
            ("POST", "/api/web-servers/1/reload"),
            ("POST", "/api/web-servers/1/configtest"),
            ("GET", "/api/web-servers/1/config"),
            ("POST", "/api/web-servers/1/switch-engine"),
            ("POST", "/api/web-servers/1/preset"),
            ("POST", "/api/web-servers/1/autostart"),
            ("POST", "/api/web-servers/native/install"),
            ("POST", "/api/web-servers/native/uninstall"),
            ("POST", "/api/web-servers/native/autostart"),
            ("GET", "/api/web-servers/native/detect"),
            ("GET", "/api/web-servers/1/native-status"),
            ("GET", "/api/web-servers/presets"),
            // settings
            ("GET", "/api/settings"),
            ("GET", "/api/settings/smtp"),
            ("PUT", "/api/settings"),
            ("PATCH", "/api/settings/batch"),
            // databases
            ("GET", "/api/databases"),
            ("DELETE", "/api/databases/1"),
            ("GET", "/api/databases/1/status"),
            ("POST", "/api/databases/1/start"),
            ("POST", "/api/databases/1/stop"),
            ("POST", "/api/databases/1/restart"),
            ("POST", "/api/databases/mysql/install"),
            ("POST", "/api/databases/redis/install"),
            ("POST", "/api/databases/1/databases"),
            ("DELETE", "/api/databases/1/databases/db1"),
            ("POST", "/api/databases/1/users"),
            ("DELETE", "/api/databases/1/users/u1"),
            ("POST", "/api/databases/1/uninstall"),
            ("PATCH", "/api/databases/batch-status"),
            // app-store
            ("GET", "/api/app-store/packages"),
            ("POST", "/api/app-store/packages/batch-import"),
            ("POST", "/api/app-store/packages/nginx/install"),
            ("POST", "/api/app-store/packages/nginx/import"),
            ("GET", "/api/app-store/installed"),
            ("GET", "/api/app-store/installed/1"),
            ("GET", "/api/app-store/installed/1/logs"),
            ("POST", "/api/app-store/installed/1/launch"),
            ("POST", "/api/app-store/installed/1/upgrade"),
            ("POST", "/api/app-store/installed/1/uninstall"),
            ("GET", "/api/app-store/wasm-builtins"),
            // files
            ("GET", "/api/files"),
            ("GET", "/api/files/read"),
            ("POST", "/api/files/write"),
            ("GET", "/api/files/download"),
            ("POST", "/api/files/upload"),
            ("POST", "/api/files/create-file"),
            ("POST", "/api/files/create-dir"),
            ("DELETE", "/api/files/delete"),
            ("POST", "/api/files/rename"),
            ("POST", "/api/files/chmod"),
            // firewall
            ("GET", "/api/firewall/rules"),
            ("POST", "/api/firewall/rules"),
            ("GET", "/api/firewall/rules/1"),
            ("PUT", "/api/firewall/rules/1"),
            ("DELETE", "/api/firewall/rules/1"),
            ("POST", "/api/firewall/rules/1/toggle"),
            ("POST", "/api/firewall/apply"),
            ("GET", "/api/firewall/status"),
            ("POST", "/api/firewall/enable"),
            ("POST", "/api/firewall/disable"),
            ("POST", "/api/firewall/reorder"),
            // scheduled tasks
            ("GET", "/api/scheduled-tasks"),
            ("POST", "/api/scheduled-tasks"),
            ("GET", "/api/scheduled-tasks/1"),
            ("PUT", "/api/scheduled-tasks/1"),
            ("DELETE", "/api/scheduled-tasks/1"),
            ("POST", "/api/scheduled-tasks/1/run"),
            ("POST", "/api/scheduled-tasks/1/toggle"),
            // tasks
            ("GET", "/api/tasks"),
            ("GET", "/api/tasks/1"),
            ("POST", "/api/tasks/1/cancel"),
            ("POST", "/api/tasks/prune"),
            // memos
            ("GET", "/api/memos"),
            ("POST", "/api/memos"),
            ("PUT", "/api/memos/1"),
            ("DELETE", "/api/memos/1"),
            // operation logs / logs / outbox
            ("GET", "/api/operation-logs"),
            ("GET", "/api/operation-logs/export"),
            ("DELETE", "/api/operation-logs/1"),
            ("GET", "/api/outbox-events"),
            ("GET", "/api/logs"),
            ("DELETE", "/api/logs/1"),
            // metrics
            ("GET", "/api/metrics/processes"),
            // backups
            ("GET", "/api/backups"),
            ("POST", "/api/backups"),
            ("DELETE", "/api/backups/app.db"),
            ("POST", "/api/backups/app.db/restore"),
        ];

        // 显式 auth-only 白名单（需登录但不做资源级 RBAC），与 middleware::is_auth_only_path 一致
        let auth_only: &[(&str, &str)] = &[
            ("GET", "/api/auth/me"),
            ("POST", "/api/auth/change-password"),
            ("POST", "/api/auth/logout"),
            ("GET", "/api/databases/1"),
            ("GET", "/api/databases/1/databases"),
        ];

        let mut missing = Vec::new();
        for &(method, path) in routes {
            let mapped =
                route_permission(&Method::from_bytes(method.as_bytes()).unwrap(), path).is_some();
            let open = auth_only.contains(&(method, path));
            if !mapped && !open {
                missing.push(format!("{} {}", method, path));
            }
        }
        assert!(
            missing.is_empty(),
            "以下受保护路由未声明权限且不在 auth-only 白名单（Stage 5 要求默认 403）：\n{}",
            missing.join("\n")
        );
    }
}
