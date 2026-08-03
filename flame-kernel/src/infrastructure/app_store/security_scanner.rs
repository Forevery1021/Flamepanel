use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanFinding {
    pub severity: Severity,
    pub message: String,
    pub item: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Block,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Default)]
pub struct ScanResult {
    pub findings: Vec<ScanFinding>,
}

impl ScanResult {
    pub fn has_blockers(&self) -> bool {
        self.findings.iter().any(|f| f.severity == Severity::Block)
    }

    pub fn block_messages(&self) -> Vec<String> {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Block)
            .map(|f| f.message.clone())
            .collect()
    }

    pub fn summary(&self) -> Vec<String> {
        self.findings.iter().map(|f| f.message.clone()).collect()
    }
}

const SENSITIVE_MOUNTS: &[&str] = &["/etc", "/root", "/boot", "/var/run/docker.sock"];

/// Compose 安全检查：返回按严重度分类的结果。
/// - Block：privileged: true（默认阻断）
/// - High：敏感路径挂载
/// - Medium：network_mode: host
/// - Low：镜像来自非白名单仓库
pub fn scan_compose(compose_yaml: &str, confirmed_risky: bool) -> ScanResult {
    let mut result = ScanResult::default();

    if compose_yaml.to_lowercase().contains("privileged: true") {
        result.findings.push(ScanFinding {
            severity: if confirmed_risky {
                Severity::High
            } else {
                Severity::Block
            },
            message: "容器以 privileged 特权模式运行，具有宿主 root 权限".into(),
            item: "privileged: true".into(),
        });
    }

    for mount in SENSITIVE_MOUNTS {
        if compose_yaml.contains(&format!("\"{}:", mount))
            || compose_yaml.contains(&format!(" - {}:", mount))
            || compose_yaml.contains(&format!("- {}:", mount))
        {
            result.findings.push(ScanFinding {
                severity: Severity::High,
                message: format!("挂载了敏感宿主机路径: {}", mount),
                item: mount.to_string(),
            });
        }
    }

    if compose_yaml.to_lowercase().contains("network_mode: host") {
        result.findings.push(ScanFinding {
            severity: Severity::Medium,
            message: "使用 host 网络模式，容器与宿主机共享网络栈".into(),
            item: "network_mode: host".into(),
        });
    }

    // 镜像仓库检查（白名单）
    let trusted_registries = [
        "docker.io",
        "ghcr.io",
        "quay.io",
        "registry.cn-hangzhou.aliyuncs.com",
    ];
    for line in compose_yaml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("image:") {
            let image = trimmed
                .trim_start_matches("image:")
                .trim()
                .trim_matches('"')
                .trim_matches('\'');
            let registry = image.split('/').next().unwrap_or(image);
            if image.contains('/')
                && !trusted_registries.contains(&registry)
                && !registry.contains('.')
                && !registry.contains(':')
            {
                // 如 "wordpress:6.7" 不带仓库前缀，视为 docker.io
                continue;
            }
            if !trusted_registries.contains(&registry)
                && registry.contains('.')
                && !registry.contains(":")
            {
                result.findings.push(ScanFinding {
                    severity: Severity::Low,
                    message: format!("镜像来自非白名单仓库: {}", image),
                    item: image.to_string(),
                });
            }
        }
    }

    if !compose_yaml.to_lowercase().contains("restart:") {
        result.findings.push(ScanFinding {
            severity: Severity::Info,
            message: "未定义 restart 策略，将自动补充 unless-stopped".into(),
            item: "restart".into(),
        });
    }

    result
}

/// 自动为没有 restart 策略的 compose 补充 `restart: unless-stopped`（仅对顶层 services 缺省处理，
/// 简单实现：若整体无 restart 且存在 services 块，追加到首个服务）
pub fn ensure_restart_policy(compose_yaml: &str) -> String {
    if compose_yaml.to_lowercase().contains("restart:") {
        return compose_yaml.to_string();
    }
    if compose_yaml.contains("services:") {
        // 在第一个服务名下缩进插入 restart: unless-stopped
        let mut lines: Vec<&str> = compose_yaml.lines().collect();
        let mut inserted = false;
        let mut i = 0;
        while i < lines.len() - 1 {
            let cur = lines[i];
            let next = lines[i + 1];
            let cur_indent = cur.len() - cur.trim_start().len();
            if cur.trim_start().starts_with('-') && next.trim().is_empty() {
                i += 1;
                continue;
            }
            if cur.trim().is_empty() {
                i += 1;
                continue;
            }
            // 找到 services 下的第一个顶层服务（缩进 2 的空格，不以空格开头的是 services: 或注释）
            let trimmed = cur.trim_start();
            if cur_indent == 2
                && !trimmed.starts_with('#')
                && !trimmed.starts_with('-')
                && !trimmed.starts_with("services:")
            {
                lines.insert(i + 1, "    restart: unless-stopped");
                inserted = true;
                break;
            }
            i += 1;
        }
        if inserted {
            return lines.join("\n");
        }
    }
    compose_yaml.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAFE_COMPOSE: &str = r#"services:
  app:
    image: nginx:alpine
    ports:
      - "8080:80"
    restart: unless-stopped
"#;

    #[test]
    fn safe_compose_has_no_findings() {
        let result = scan_compose(SAFE_COMPOSE, false);
        assert!(!result.has_blockers());
        assert!(result.findings.is_empty());
    }

    #[test]
    fn privileged_blocks_by_default() {
        let yaml = "services:\n  app:\n    image: busybox\n    privileged: true\n";
        let result = scan_compose(yaml, false);
        assert!(result.has_blockers());
        assert_eq!(result.block_messages().len(), 1);
    }

    #[test]
    fn privileged_passes_when_confirmed() {
        let yaml = "services:\n  app:\n    image: busybox\n    privileged: true\n";
        let result = scan_compose(yaml, true);
        assert!(!result.has_blockers());
    }

    #[test]
    fn sensitive_mount_flagged_high() {
        let yaml = "services:\n  app:\n    image: busybox\n    volumes:\n      - /etc:/host/etc\n";
        let result = scan_compose(yaml, false);
        assert!(!result.has_blockers());
        assert!(result
            .findings
            .iter()
            .any(|f| f.severity == Severity::High && f.item == "/etc"));
    }

    #[test]
    fn host_network_flagged_medium() {
        let yaml = "services:\n  app:\n    image: busybox\n    network_mode: host\n";
        let result = scan_compose(yaml, false);
        assert!(result
            .findings
            .iter()
            .any(|f| f.severity == Severity::Medium));
    }

    #[test]
    fn unknown_registry_flagged_low() {
        let yaml = "services:\n  app:\n    image: myregistry.example.com/myapp:1.0\n";
        let result = scan_compose(yaml, false);
        assert!(result.findings.iter().any(|f| f.severity == Severity::Low));
    }

    #[test]
    fn missing_restart_gets_info() {
        let yaml = "services:\n  app:\n    image: nginx:alpine\n";
        let result = scan_compose(yaml, false);
        assert!(result.findings.iter().any(|f| f.severity == Severity::Info));
    }

    #[test]
    fn ensure_restart_policy_adds_policy() {
        let yaml = "services:\n  app:\n    image: nginx:alpine\n    ports:\n      - \"8080:80\"\n";
        let out = ensure_restart_policy(yaml);
        assert!(out.contains("restart: unless-stopped"));
        assert!(out.contains("services:"));
    }

    #[test]
    fn ensure_restart_policy_keeps_existing() {
        let out = ensure_restart_policy(SAFE_COMPOSE);
        assert_eq!(out, SAFE_COMPOSE);
    }
}
