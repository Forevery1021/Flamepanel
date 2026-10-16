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
    // DNS 厂商账号（主线三 §6.1 · Stage 4）：凭据托管与连通性测试
    PermissionRule::exact("GET", "/api/dns-providers", "dns_provider", "read"),
    PermissionRule::prefix("GET", "/api/dns-providers/", "dns_provider", "read"),
    PermissionRule::exact("POST", "/api/dns-providers", "dns_provider", "create"),
    PermissionRule::pre_suf(
        "POST",
        "/api/dns-providers/",
        "/test",
        "dns_provider",
        "update",
    ),
    PermissionRule::prefix("PUT", "/api/dns-providers/", "dns_provider", "update"),
    PermissionRule::prefix("DELETE", "/api/dns-providers/", "dns_provider", "delete"),
    // SSL 证书自动化（主线三 §6.1 · Stage 2）：issue/upload 放 update 动作
    PermissionRule::exact("GET", "/api/certificates", "certificate", "read"),
    // S-10 续签历史（显式登记；与下方前缀规则同口径）
    PermissionRule::pre_suf(
        "GET",
        "/api/certificates/",
        "/renewals",
        "certificate",
        "read",
    ),
    PermissionRule::prefix("GET", "/api/certificates/", "certificate", "read"),
    PermissionRule::exact("POST", "/api/certificates", "certificate", "update"),
    PermissionRule::exact(
        "POST",
        "/api/certificates/issue/acme",
        "certificate",
        "update",
    ),
    PermissionRule::exact(
        "POST",
        "/api/certificates/issue/self-signed",
        "certificate",
        "update",
    ),
    PermissionRule::prefix("PUT", "/api/certificates/", "certificate", "update"),
    PermissionRule::pre_suf(
        "POST",
        "/api/certificates/",
        "/bind",
        "certificate",
        "update",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/certificates/",
        "/renew",
        "certificate",
        "update",
    ),
    PermissionRule::prefix("DELETE", "/api/certificates/", "certificate", "delete"),
    // websites
    PermissionRule::exact("GET", "/api/websites", "website", "read"),
    PermissionRule::exact("POST", "/api/websites", "website", "create"),
    PermissionRule::prefix("GET", "/api/websites/", "website", "read"),
    PermissionRule::prefix("PUT", "/api/websites/", "website", "update"),
    // 清除伪静态套用（Doc/20 §5.5 收口）：站点配置变更语义，须先于通用 delete 前缀命中
    PermissionRule::pre_suf("DELETE", "/api/websites/", "/rewrite", "website", "update"),
    PermissionRule::prefix("DELETE", "/api/websites/", "website", "delete"),
    // WAF 基础防护（主线三 §6.2）：规则 CRUD + 拦截日志查询/清理 + 引擎诊断
    PermissionRule::exact("GET", "/api/waf/rules", "waf", "read"),
    PermissionRule::prefix("GET", "/api/waf/rules/", "waf", "read"),
    PermissionRule::exact("POST", "/api/waf/rules", "waf", "create"),
    PermissionRule::prefix("PUT", "/api/waf/rules/", "waf", "update"),
    PermissionRule::prefix("DELETE", "/api/waf/rules/", "waf", "delete"),
    PermissionRule::exact("GET", "/api/waf/logs", "waf", "read"),
    PermissionRule::exact("GET", "/api/waf/status", "waf", "read"),
    PermissionRule::exact("POST", "/api/waf/logs/clear", "waf", "update"),
    PermissionRule::exact("POST", "/api/waf/apply", "waf", "update"),
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
        "GET",
        "/api/docker/containers/",
        "/compose",
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
    // 部署任意 compose YAML 等价于宿主机 root（privileged / 挂载 `/`），归入 create 而非「启动容器」
    PermissionRule::exact("POST", "/api/docker/compose/deploy", "docker", "create"),
    // docker run → compose 纯文本转换：结果只用于随后的部署，与部署同属 create
    PermissionRule::exact(
        "POST",
        "/api/docker/compose/convert-run",
        "docker",
        "create",
    ),
    PermissionRule::pre_suf("POST", "/api/docker/compose/", "/up", "docker", "start"),
    PermissionRule::pre_suf("POST", "/api/docker/compose/", "/down", "docker", "stop"),
    // plugins
    // 插件市场（batch-14）：须排在 `/api/plugins/` 通配规则之前（POST 通配会落到 delete）
    PermissionRule::exact("GET", "/api/plugins/market", "plugin", "read"),
    PermissionRule::pre_suf(
        "POST",
        "/api/plugins/market/",
        "/install",
        "plugin",
        "create",
    ),
    // 变更宿主能力授予 = 与加载插件同级的授权动作
    PermissionRule::pre_suf("PUT", "/api/plugins/", "/capabilities", "plugin", "create"),
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
    PermissionRule::exact("GET", "/api/web-servers/capabilities", "web_server", "read"),
    PermissionRule::exact(
        "POST",
        "/api/web-servers/capabilities/install",
        "web_server",
        "update",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/websites/",
        "/switch-engine",
        "website",
        "update",
    ),
    // 站点配置流水线：手动部署（Doc/20 §5.5）
    PermissionRule::pre_suf("POST", "/api/websites/", "/deploy", "website", "update"),
    // websites 绑定闭环（M-B 主线二 Stage A-6 · Doc/20 §5.5）
    PermissionRule::pre_suf("POST", "/api/websites/", "/runtime", "website", "update"),
    PermissionRule::pre_suf("POST", "/api/websites/", "/rewrite", "website", "update"),
    PermissionRule::pre_suf("GET", "/api/websites/", "/rewrite", "website", "read"),
    PermissionRule::pre_suf("GET", "/api/runtimes/", "/sites", "website", "read"),
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
    // M-E 只读 IaC 导出：下载入口 settings:read，各区块由 handler 独立检查资源 read。
    PermissionRule::exact("GET", "/api/iac/export", "settings", "read"),
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
    // 远程应用源（M-A Issue 1.2，Doc/20 §4.1）
    PermissionRule::exact("GET", "/api/app-store/sources", "app_store", "read"),
    PermissionRule::pre_not_suffix(
        "GET",
        "/api/app-store/sources/",
        &["/sync", "/packages"],
        "app_store",
        "read",
    ),
    PermissionRule::pre_suf(
        "GET",
        "/api/app-store/sources/",
        "/packages",
        "app_store",
        "read",
    ),
    PermissionRule::exact("POST", "/api/app-store/sources", "app_store", "create"),
    PermissionRule::pre_suf(
        "POST",
        "/api/app-store/sources/",
        "/sync",
        "app_store",
        "update",
    ),
    PermissionRule::prefix("PUT", "/api/app-store/sources/", "app_store", "update"),
    PermissionRule::prefix("DELETE", "/api/app-store/sources/", "app_store", "delete"),
    // 商店安全配置（Doc/20 §4.2 收口）：验签策略 / 镜像仓库白名单
    PermissionRule::exact("GET", "/api/app-store/security", "app_store", "read"),
    PermissionRule::exact("PUT", "/api/app-store/security", "app_store", "update"),
    PermissionRule::exact("GET", "/api/app-store/installed", "app_store", "read"),
    PermissionRule::pre_not_suffix(
        "GET",
        "/api/app-store/installed/",
        &["/upgrade", "/uninstall", "/logs", "/upgrade-preview"],
        "app_store",
        "read",
    ),
    PermissionRule::pre_suf(
        "GET",
        "/api/app-store/installed/",
        "/upgrade-preview",
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
    // 原生软件生命周期（Doc/16 §6.5：status/start/stop/restart）
    PermissionRule::pre_suf(
        "GET",
        "/api/app-store/installed/",
        "/native-status",
        "app_store",
        "read",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/app-store/installed/",
        "/native-start",
        "app_store",
        "update",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/app-store/installed/",
        "/native-stop",
        "app_store",
        "update",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/app-store/installed/",
        "/native-restart",
        "app_store",
        "update",
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
    // 实例详情 / 状态 / 子库列表均为读。旧规则用 `pre_not_contains(.., "/databases/")` 排除子路径，
    // 但整条路径本身就以 `/api/databases/` 开头——规则永不命中，两个端点曾被迫挂进 auth-only 白名单，
    // 任何登录用户（含无 database 权限的自定义角色）都能读实例路径与库名。
    PermissionRule::prefix("GET", "/api/databases/", "database", "read"),
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
    PermissionRule::exact("POST", "/api/files/copy", "file", "write"),
    PermissionRule::exact("POST", "/api/files/move", "file", "write"),
    PermissionRule::exact("POST", "/api/files/upload", "file", "upload"),
    // files · 压缩 / 解压 / 回收站（M-C 文件管理收尾；回收站的还原与清除视同写操作）
    PermissionRule::exact("POST", "/api/files/compress", "file", "write"),
    PermissionRule::exact("POST", "/api/files/extract", "file", "write"),
    PermissionRule::exact("GET", "/api/files/recycle", "file", "read"),
    PermissionRule::pre_suf("POST", "/api/files/recycle/", "/restore", "file", "write"),
    PermissionRule::prefix("DELETE", "/api/files/recycle", "file", "write"),
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
    // backup accounts（备份账号，M-A 收尾 Stage 1 · Doc/20 §6.5）
    // 注意：必须先于 backups 的 prefix 规则，避免 /api/backup-accounts 被 /api/backups/ 前缀抢先映射
    PermissionRule::exact("GET", "/api/backup-accounts", "backup", "read"),
    PermissionRule::pre_not_suffix("GET", "/api/backup-accounts/", &["/test"], "backup", "read"),
    PermissionRule::exact("POST", "/api/backup-accounts", "backup", "create"),
    PermissionRule::pre_suf("POST", "/api/backup-accounts/", "/test", "backup", "update"),
    // Stage 2：云端备份对象管理（上传到云端账号 / 列表 / 删除）
    PermissionRule::pre_suf(
        "POST",
        "/api/backup-accounts/",
        "/objects/upload",
        "backup",
        "create",
    ),
    PermissionRule::prefix("PUT", "/api/backup-accounts/", "backup", "update"),
    PermissionRule::prefix("DELETE", "/api/backup-accounts/", "backup", "delete"),
    // backup jobs（计划备份任务，M-A 收尾 Stage 3 · Doc/20 §6.5）
    // 注意：必须先于 backups 的 prefix 规则，避免 /api/backup-jobs 被 /api/backups/ 前缀抢先映射
    PermissionRule::exact("GET", "/api/backup-jobs", "backup", "read"),
    PermissionRule::exact("POST", "/api/backup-jobs", "backup", "create"),
    PermissionRule::pre_suf("POST", "/api/backup-jobs/", "/run", "backup", "update"),
    PermissionRule::pre_not_suffix("POST", "/api/backup-jobs/", &["/run"], "backup", "create"),
    PermissionRule::prefix("PUT", "/api/backup-jobs/", "backup", "update"),
    PermissionRule::prefix("DELETE", "/api/backup-jobs/", "backup", "delete"),
    // metrics
    PermissionRule::exact("GET", "/api/metrics/processes", "node", "read"),
    PermissionRule::exact("GET", "/api/metrics/history", "node", "read"),
    // inspection（S-17 系统体检：toolbox 域）
    PermissionRule::exact("GET", "/api/inspection", "toolbox", "read"),
    PermissionRule::exact("POST", "/api/inspection/run", "toolbox", "execute"),
    // 内核日志（W0 · Doc/32 §4）：读 / 下载 = log:read，删除历史文件 = log:delete，运行时调级 = settings:update
    PermissionRule::exact("GET", "/api/kernel-logs", "log", "read"),
    PermissionRule::exact("GET", "/api/kernel-logs/entries", "log", "read"),
    PermissionRule::exact("GET", "/api/kernel-logs/stats", "log", "read"),
    PermissionRule::prefix("GET", "/api/kernel-logs/files/", "log", "read"),
    PermissionRule::prefix("DELETE", "/api/kernel-logs/files/", "log", "delete"),
    PermissionRule::exact("PUT", "/api/kernel-logs/level", "settings", "update"),
    // M-AI-1 AI 接入中心（协议端点 `/api/mcp` 由 auth_middleware 以 fpk_ 凭证单独鉴权）
    PermissionRule::exact("GET", "/api/mcp/tools", "mcp", "read"),
    PermissionRule::exact("GET", "/api/mcp/prompts", "mcp", "read"),
    PermissionRule::prefix("GET", "/api/mcp/prompts/", "mcp", "read"),
    PermissionRule::exact("GET", "/api/mcp/principals", "mcp", "read"),
    PermissionRule::exact("POST", "/api/mcp/principals", "mcp", "create"),
    PermissionRule::pre_suf("POST", "/api/mcp/principals/", "/revoke", "mcp", "create"),
    PermissionRule::exact("GET", "/api/mcp/sessions", "mcp", "read"),
    PermissionRule::exact("GET", "/api/mcp/calls", "mcp", "read"),
    // M-AI-2 写操作审批（工具本身权限 / 高危 mcp:delete 由 handler 字段级校验）
    PermissionRule::exact("GET", "/api/mcp/approvals", "mcp", "read"),
    PermissionRule::pre_suf("POST", "/api/mcp/approvals/", "/approve", "mcp", "update"),
    PermissionRule::pre_suf("POST", "/api/mcp/approvals/", "/reject", "mcp", "update"),
    PermissionRule::prefix("GET", "/api/mcp/approvals/", "mcp", "read"),
    // M-AI-3 面板内置助手：查看配置 = mcp:read；改配置 / 测试连接 = settings:update；
    // 对话 = mcp:create（消耗模型额度，且会按需签发本人的内置凭证）。工具级权限在对话内按角色逐个校验。
    PermissionRule::exact("GET", "/api/assistant/config", "mcp", "read"),
    PermissionRule::exact("PUT", "/api/assistant/config", "settings", "update"),
    PermissionRule::exact("POST", "/api/assistant/test", "settings", "update"),
    PermissionRule::exact("POST", "/api/assistant/chat", "mcp", "create"),
    // toolbox 进程管理（S-12）
    PermissionRule::exact("GET", "/api/toolbox/processes", "toolbox", "read"),
    PermissionRule::exact("GET", "/api/toolbox/host", "toolbox", "read"),
    PermissionRule::pre_suf(
        "POST",
        "/api/toolbox/processes/",
        "/kill",
        "toolbox",
        "execute",
    ),
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
    // projects（M-B 主线二 Stage A-3 · Doc/20 §5.6）
    PermissionRule::exact("GET", "/api/projects", "project", "read"),
    PermissionRule::pre_not_suffix("GET", "/api/projects/", &["/logs"], "project", "read"),
    PermissionRule::pre_suf("GET", "/api/projects/", "/logs", "project", "read"),
    // Procfile 进程声明查看（M-B 主线二 Stage A-8 · Doc/20 §5.8 ③）
    PermissionRule::pre_suf("GET", "/api/projects/", "/procfile", "project", "read"),
    // 构建器决策查看（M-B 主线二 Stage A-8 · Doc/20 §5.8 ②）
    PermissionRule::pre_suf("GET", "/api/projects/", "/builder", "project", "read"),
    // 服务联链（M-B 主线二 Stage A-10 · Doc/20 §5.8 ⑤）
    PermissionRule::pre_suf("GET", "/api/projects/", "/links", "project", "read"),
    PermissionRule::pre_suf("POST", "/api/projects/", "/links", "project", "update"),
    PermissionRule::pre_suf("DELETE", "/api/projects/", "/links", "project", "update"),
    PermissionRule::pre_suf(
        "POST",
        "/api/projects/",
        "/health-check",
        "project",
        "update",
    ),
    PermissionRule::exact("POST", "/api/projects", "project", "create"),
    PermissionRule::pre_suf("POST", "/api/projects/", "/deploy", "project", "create"),
    PermissionRule::pre_suf("POST", "/api/projects/", "/start", "project", "update"),
    PermissionRule::pre_suf("POST", "/api/projects/", "/stop", "project", "update"),
    PermissionRule::pre_suf("POST", "/api/projects/", "/restart", "project", "update"),
    PermissionRule::prefix("PUT", "/api/projects/", "project", "update"),
    PermissionRule::prefix("DELETE", "/api/projects/", "project", "delete"),
    // webhook 部署通道管理（M-B 主线二 Stage A-7 · Doc/20 §5.8）
    // POST /api/projects/:id/webhook 本体免 JWT（部署令牌守卫在 auth_middleware），
    // 且规则表在 auth_middleware 之前不会到达（守卫提前返回），无需声明；
    // enable/disable 属敏感配置操作，挂 project:update。
    PermissionRule::pre_suf(
        "POST",
        "/api/projects/",
        "/webhook/enable",
        "project",
        "update",
    ),
    PermissionRule::pre_suf(
        "POST",
        "/api/projects/",
        "/webhook/disable",
        "project",
        "update",
    ),
    // runtimes（M-B 主线二 Stage A-3 · Doc/20 §5.6）
    PermissionRule::exact("GET", "/api/runtimes", "runtime", "read"),
    PermissionRule::prefix("GET", "/api/runtimes/", "runtime", "read"),
    PermissionRule::exact("POST", "/api/runtimes/install", "runtime", "create"),
    PermissionRule::pre_suf("POST", "/api/runtimes/", "/default", "runtime", "update"),
    PermissionRule::prefix("DELETE", "/api/runtimes/", "runtime", "delete"),
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
        assert_eq!(
            perm("PUT", "/api/users/1/2fa/reset"),
            Some(("user", "update"))
        );
        assert_eq!(perm("POST", "/api/auth/2fa/setup"), None);
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
        // certificates
        assert_eq!(
            perm("GET", "/api/certificates"),
            Some(("certificate", "read"))
        );
        assert_eq!(
            perm("POST", "/api/certificates/issue/acme"),
            Some(("certificate", "update"))
        );
        assert_eq!(
            perm("POST", "/api/certificates/3/bind"),
            Some(("certificate", "update"))
        );
        assert_eq!(
            perm("POST", "/api/certificates/3/renew"),
            Some(("certificate", "update"))
        );
        assert_eq!(
            perm("GET", "/api/certificates/3/renewals"),
            Some(("certificate", "read"))
        );
        assert_eq!(
            perm("DELETE", "/api/certificates/3"),
            Some(("certificate", "delete"))
        );
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
            Some(("docker", "create"))
        );
        assert_eq!(
            perm("POST", "/api/docker/compose/convert-run"),
            Some(("docker", "create"))
        );
        assert_eq!(
            perm("GET", "/api/docker/containers/web/compose"),
            Some(("docker", "read"))
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
        assert_eq!(perm("GET", "/api/plugins/market"), Some(("plugin", "read")));
        assert_eq!(
            perm("POST", "/api/plugins/market/flame-hello/install"),
            Some(("plugin", "create"))
        );
        assert_eq!(
            perm("PUT", "/api/plugins/p1/capabilities"),
            Some(("plugin", "create"))
        );
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
        assert_eq!(perm("GET", "/api/databases/1"), Some(("database", "read")));
        assert_eq!(
            perm("GET", "/api/databases/1/status"),
            Some(("database", "read"))
        );
        assert_eq!(
            perm("GET", "/api/databases/1/databases"),
            Some(("database", "read"))
        );
        assert_eq!(
            perm("POST", "/api/databases/mysql/install"),
            Some(("database", "create"))
        );
        assert_eq!(
            perm("POST", "/api/databases/mongodb/install"),
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
        assert_eq!(perm("POST", "/api/files/compress"), Some(("file", "write")));
        assert_eq!(perm("POST", "/api/files/extract"), Some(("file", "write")));
        assert_eq!(perm("GET", "/api/files/recycle"), Some(("file", "read")));
        assert_eq!(
            perm("POST", "/api/files/recycle/1-abc/restore"),
            Some(("file", "write"))
        );
        assert_eq!(
            perm("DELETE", "/api/files/recycle/1-abc"),
            Some(("file", "write"))
        );
        assert_eq!(
            perm("DELETE", "/api/files/recycle"),
            Some(("file", "write"))
        );
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
            perm("GET", "/api/app-store/installed/1/upgrade-preview"),
            Some(("app_store", "read"))
        );
        assert_eq!(
            perm("POST", "/api/app-store/installed/1/launch"),
            Some(("app_store", "read"))
        );
        assert_eq!(
            perm("POST", "/api/app-store/installed/1/uninstall"),
            Some(("app_store", "delete"))
        );
        // 商店安全配置（Doc/20 §4.2 收口）
        assert_eq!(
            perm("GET", "/api/app-store/security"),
            Some(("app_store", "read"))
        );
        assert_eq!(
            perm("PUT", "/api/app-store/security"),
            Some(("app_store", "update"))
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
        // 内核日志（W0）：下载归 log:read，删除归 log:delete，调级归 settings:update
        assert_eq!(
            perm("GET", "/api/kernel-logs/entries"),
            Some(("log", "read"))
        );
        assert_eq!(
            perm("GET", "/api/kernel-logs/files/a.log/download"),
            Some(("log", "read"))
        );
        assert_eq!(
            perm("DELETE", "/api/kernel-logs/files/a.log"),
            Some(("log", "delete"))
        );
        assert_eq!(
            perm("PUT", "/api/kernel-logs/level"),
            Some(("settings", "update"))
        );
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
    fn project_webhook_permissions() {
        assert_eq!(
            perm("POST", "/api/projects/1/webhook/enable"),
            Some(("project", "update"))
        );
        assert_eq!(
            perm("POST", "/api/projects/1/webhook/disable"),
            Some(("project", "update"))
        );
    }

    #[test]
    fn website_rewrite_persistence_permissions() {
        assert_eq!(
            perm("GET", "/api/websites/1/rewrite"),
            Some(("website", "read"))
        );
        assert_eq!(
            perm("POST", "/api/websites/1/rewrite"),
            Some(("website", "update"))
        );
        assert_eq!(
            perm("DELETE", "/api/websites/1/rewrite"),
            Some(("website", "update"))
        );
        // 站点配置流水线：手动部署需要 website:update
        assert_eq!(
            perm("POST", "/api/websites/1/deploy"),
            Some(("website", "update"))
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
            ("PUT", "/api/users/1/2fa/reset"),
            // auth（仅旋转密钥受 RBAC 保护；其余为认证自服务）
            ("POST", "/api/auth/rotate-secret"),
            ("GET", "/api/auth/2fa"),
            ("POST", "/api/auth/2fa/setup"),
            ("POST", "/api/auth/2fa/enable"),
            ("POST", "/api/auth/2fa/disable"),
            ("POST", "/api/auth/2fa/recovery-codes"),
            // dns providers（主线三 §6.1 · Stage 4）
            ("GET", "/api/dns-providers"),
            ("POST", "/api/dns-providers"),
            ("PUT", "/api/dns-providers/1"),
            ("DELETE", "/api/dns-providers/1"),
            ("POST", "/api/dns-providers/1/test"),
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
            ("POST", "/api/websites/1/deploy"),
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
            ("POST", "/api/docker/compose/convert-run"),
            ("GET", "/api/docker/containers/c1/compose"),
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
            ("GET", "/api/databases/1"),
            ("GET", "/api/databases/1/databases"),
            ("DELETE", "/api/databases/1"),
            ("GET", "/api/databases/1/status"),
            ("POST", "/api/databases/1/start"),
            ("POST", "/api/databases/1/stop"),
            ("POST", "/api/databases/1/restart"),
            ("POST", "/api/databases/mysql/install"),
            ("POST", "/api/databases/redis/install"),
            ("POST", "/api/databases/postgres/install"),
            ("POST", "/api/databases/mongodb/install"),
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
            ("GET", "/api/app-store/installed/1/upgrade-preview"),
            ("POST", "/api/app-store/installed/1/uninstall"),
            ("GET", "/api/app-store/wasm-builtins"),
            ("GET", "/api/app-store/sources"),
            ("POST", "/api/app-store/sources"),
            ("PUT", "/api/app-store/sources/1"),
            ("DELETE", "/api/app-store/sources/1"),
            ("POST", "/api/app-store/sources/1/sync"),
            ("GET", "/api/app-store/sources/1/packages"),
            ("GET", "/api/app-store/security"),
            ("PUT", "/api/app-store/security"),
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
            ("POST", "/api/files/compress"),
            ("POST", "/api/files/extract"),
            ("GET", "/api/files/recycle"),
            ("POST", "/api/files/recycle/1/restore"),
            ("DELETE", "/api/files/recycle/1"),
            ("DELETE", "/api/files/recycle"),
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
            // inspection（S-17）
            ("GET", "/api/inspection"),
            ("POST", "/api/inspection/run"),
            // 内核日志（W0）
            ("GET", "/api/kernel-logs"),
            ("GET", "/api/kernel-logs/entries"),
            ("GET", "/api/kernel-logs/stats"),
            (
                "GET",
                "/api/kernel-logs/files/flamepanel.2026-09-25.log/download",
            ),
            ("DELETE", "/api/kernel-logs/files/flamepanel.2026-09-24.log"),
            ("PUT", "/api/kernel-logs/level"),
            // AI 接入中心（M-AI-1）
            ("GET", "/api/mcp/tools"),
            ("GET", "/api/mcp/prompts"),
            ("GET", "/api/mcp/prompts/container_diagnose"),
            ("GET", "/api/mcp/principals"),
            ("POST", "/api/mcp/principals"),
            ("POST", "/api/mcp/principals/1/revoke"),
            ("GET", "/api/mcp/sessions"),
            ("GET", "/api/mcp/calls"),
            ("GET", "/api/mcp/approvals"),
            ("GET", "/api/mcp/approvals/1"),
            ("POST", "/api/mcp/approvals/1/approve"),
            ("POST", "/api/mcp/approvals/1/reject"),
            ("GET", "/api/assistant/config"),
            ("PUT", "/api/assistant/config"),
            ("POST", "/api/assistant/test"),
            ("POST", "/api/assistant/chat"),
            // toolbox 进程管理（S-12）
            ("GET", "/api/toolbox/processes"),
            ("GET", "/api/toolbox/host"),
            ("POST", "/api/toolbox/processes/1234/kill"),
            ("GET", "/api/metrics/history"),
            // projects（M-B 主线二 Stage A-3）
            ("GET", "/api/projects"),
            ("POST", "/api/projects"),
            ("GET", "/api/projects/1"),
            ("PUT", "/api/projects/1"),
            ("DELETE", "/api/projects/1"),
            ("POST", "/api/projects/1/deploy"),
            ("POST", "/api/projects/1/start"),
            ("POST", "/api/projects/1/stop"),
            ("POST", "/api/projects/1/restart"),
            ("GET", "/api/projects/1/status"),
            ("GET", "/api/projects/1/procfile"),
            ("GET", "/api/projects/1/builder"),
            ("GET", "/api/projects/1/links"),
            ("POST", "/api/projects/1/links"),
            ("DELETE", "/api/projects/1/links/1"),
            ("GET", "/api/projects/1/logs"),
            // webhook 部署通道（Stage A-7）
            ("POST", "/api/projects/1/webhook/enable"),
            ("POST", "/api/projects/1/webhook/disable"),
            // websites 绑定闭环（M-B 主线二 Stage A-6）
            ("POST", "/api/websites/1/runtime"),
            ("POST", "/api/websites/1/rewrite"),
            // 伪静态套用持久化（Doc/20 §5.5 收口）
            ("GET", "/api/websites/1/rewrite"),
            ("DELETE", "/api/websites/1/rewrite"),
            ("GET", "/api/runtimes/1/sites"),
            // runtimes（M-B 主线二 Stage A-3）
            ("GET", "/api/runtimes"),
            ("GET", "/api/runtimes/1"),
            ("GET", "/api/runtimes/available?kind=php"),
            ("GET", "/api/runtimes/kind/php"),
            ("POST", "/api/runtimes/install"),
            ("DELETE", "/api/runtimes/1"),
            ("POST", "/api/runtimes/1/default"),
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
            // Doc/33 TOTP 自助端点
            ("GET", "/api/auth/2fa"),
            ("POST", "/api/auth/2fa/setup"),
            ("POST", "/api/auth/2fa/enable"),
            ("POST", "/api/auth/2fa/disable"),
            ("POST", "/api/auth/2fa/recovery-codes"),
            // S-06：顶栏聚合徽标（handler 内按角色逐分组裁剪）
            ("GET", "/api/overview/badges"),
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

#[cfg(test)]
mod backup_account_permission_tests {
    use super::*;

    fn perm(method: &str, path: &str) -> Option<(&'static str, &'static str)> {
        route_permission(
            &axum::http::Method::from_bytes(method.as_bytes()).unwrap(),
            path,
        )
    }

    #[test]
    fn backup_account_permissions() {
        assert_eq!(
            perm("GET", "/api/backup-accounts"),
            Some(("backup", "read"))
        );
        assert_eq!(
            perm("GET", "/api/backup-accounts/3"),
            Some(("backup", "read"))
        );
        assert_eq!(
            perm("POST", "/api/backup-accounts"),
            Some(("backup", "create"))
        );
        assert_eq!(
            perm("POST", "/api/backup-accounts/3/test"),
            Some(("backup", "update"))
        );
        assert_eq!(
            perm("PUT", "/api/backup-accounts/3"),
            Some(("backup", "update"))
        );
        assert_eq!(
            perm("DELETE", "/api/backup-accounts/3"),
            Some(("backup", "delete"))
        );
        // 不被 backups 前缀规则抢先映射
        assert_eq!(perm("GET", "/api/backups"), Some(("backup", "read")));
    }
}

#[cfg(test)]
mod dns_provider_permission_tests {
    use super::*;

    fn perm(method: &str, path: &str) -> Option<(&'static str, &'static str)> {
        route_permission(
            &axum::http::Method::from_bytes(method.as_bytes()).unwrap(),
            path,
        )
    }

    #[test]
    fn mcp_management_permissions() {
        use axum::http::Method;
        assert_eq!(
            route_permission(&Method::GET, "/api/mcp/principals"),
            Some(("mcp", "read"))
        );
        assert_eq!(
            route_permission(&Method::POST, "/api/mcp/principals"),
            Some(("mcp", "create"))
        );
        assert_eq!(
            route_permission(&Method::POST, "/api/mcp/principals/3/revoke"),
            Some(("mcp", "create"))
        );
        assert_eq!(
            route_permission(&Method::GET, "/api/mcp/calls"),
            Some(("mcp", "read"))
        );
        assert_eq!(
            route_permission(&Method::GET, "/api/mcp/approvals/7"),
            Some(("mcp", "read"))
        );
        assert_eq!(
            route_permission(&Method::POST, "/api/mcp/approvals/7/approve"),
            Some(("mcp", "update"))
        );
        assert_eq!(
            route_permission(&Method::POST, "/api/mcp/approvals/7/reject"),
            Some(("mcp", "update"))
        );
        // 协议端点不走 REST 权限表（由 fpk_ 凭证鉴权）
        assert_eq!(route_permission(&Method::POST, "/api/mcp"), None);
    }

    #[test]
    fn dns_provider_permissions() {
        assert_eq!(
            perm("GET", "/api/dns-providers"),
            Some(("dns_provider", "read"))
        );
        assert_eq!(
            perm("GET", "/api/dns-providers/3"),
            Some(("dns_provider", "read"))
        );
        assert_eq!(
            perm("POST", "/api/dns-providers"),
            Some(("dns_provider", "create"))
        );
        assert_eq!(
            perm("POST", "/api/dns-providers/3/test"),
            Some(("dns_provider", "update"))
        );
        assert_eq!(
            perm("PUT", "/api/dns-providers/3"),
            Some(("dns_provider", "update"))
        );
        assert_eq!(
            perm("DELETE", "/api/dns-providers/3"),
            Some(("dns_provider", "delete"))
        );
    }
}
