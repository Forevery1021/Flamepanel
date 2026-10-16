//! `docker run` 命令 / 既有容器 → Compose YAML 转换（Doc/30 M-D「`docker run` 粘贴导入 + 存量容器收编」）。
//!
//! 纯函数：只做文本 / JSON → YAML 的转换，不接触 Docker。生成的 YAML 交给用户审阅后
//! 走既有的 `POST /api/docker/compose/deploy`（`docker:create` + 大小 / 项目名校验），
//! 因此这里不需要、也不应该做「安全放行」判断——只保证**如实**转换，并把无法等价
//! 表达的参数写进 `warnings`，而不是悄悄丢掉。
//!
//! 设计要点：
//! - 分词按 POSIX shell 引号规则（单引号字面、双引号可转义、反斜杠续行），**不执行**
//!   任何展开：遇到管道 / 重定向 / `;` / `&&` / 命令替换直接拒绝；
//! - Compose 会对 `$VAR` 做插值：shell 中字面的 `$`（单引号内或 `\$`）转写为 `$$`，
//!   未加引号的 `$VAR` 保留原样并给出警告；
//! - 未知参数一律报错（无法判断其后是否带值，继续解析只会把镜像名认错）；
//! - YAML 由 `serde_yaml` 序列化，值中的任何字符都不会破坏结构。

use serde_json::Value as Json;
use serde_yaml::{Mapping, Number, Value};

use crate::core::error::AppError;

/// 粘贴命令长度上限。
pub const MAX_RUN_COMMAND_BYTES: usize = 16 * 1024;

/// 转换结果：建议的项目名 / 服务名、YAML 与需人工确认的提示。
#[derive(Debug, Clone, PartialEq, serde::Serialize, utoipa::ToSchema)]
pub struct ComposeConversion {
    /// 建议的 Compose 项目名（满足 `validate_project_name`）
    pub project_name: String,
    /// 生成的服务名
    pub service_name: String,
    /// Compose YAML
    pub compose_yaml: String,
    /// 无法等价转换或需要人工确认的事项
    pub warnings: Vec<String>,
}

fn bad(msg: impl Into<String>) -> AppError {
    AppError::ValidationError(msg.into())
}

// ─── 分词 ────────────────────────────────────────────────────────────────────

const SHELL_OPS_MSG: &str =
    "仅支持单条 docker run 命令：不支持管道、重定向、`;`、`&&`、后台 `&` 与命令替换";

struct Tokens {
    words: Vec<String>,
    /// 出现了会被 Compose 插值的 `$`（未加引号或双引号内）
    interpolates: bool,
}

fn tokenize(input: &str) -> Result<Tokens, AppError> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut in_word = false;
    let mut interpolates = false;
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('\n') => {}
                Some('\r') => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                }
                Some('$') => {
                    cur.push_str("$$");
                    in_word = true;
                }
                Some(n) => {
                    cur.push(n);
                    in_word = true;
                }
                None => {}
            },
            '\'' => {
                in_word = true;
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some('$') => cur.push_str("$$"),
                        Some(ch) => cur.push(ch),
                        None => return Err(bad("单引号未闭合")),
                    }
                }
            }
            '"' => {
                in_word = true;
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some('$') => cur.push_str("$$"),
                            Some(ch @ ('"' | '\\' | '`')) => cur.push(ch),
                            Some('\n') => {}
                            Some(ch) => {
                                cur.push('\\');
                                cur.push(ch);
                            }
                            None => return Err(bad("双引号未闭合")),
                        },
                        Some('`') => return Err(bad(SHELL_OPS_MSG)),
                        Some('$') => {
                            if chars.peek() == Some(&'(') {
                                return Err(bad(SHELL_OPS_MSG));
                            }
                            interpolates = true;
                            cur.push('$');
                        }
                        Some(ch) => cur.push(ch),
                        None => return Err(bad("双引号未闭合")),
                    }
                }
            }
            '#' if !in_word => {
                // shell 注释：跳到行尾
                for ch in chars.by_ref() {
                    if ch == '\n' {
                        break;
                    }
                }
            }
            c if c.is_whitespace() => {
                if in_word {
                    words.push(std::mem::take(&mut cur));
                    in_word = false;
                }
            }
            ';' | '&' | '|' | '<' | '>' | '`' => return Err(bad(SHELL_OPS_MSG)),
            '$' => {
                if chars.peek() == Some(&'(') {
                    return Err(bad(SHELL_OPS_MSG));
                }
                interpolates = true;
                cur.push('$');
                in_word = true;
            }
            _ => {
                cur.push(c);
                in_word = true;
            }
        }
    }
    if in_word {
        words.push(cur);
    }
    Ok(Tokens {
        words,
        interpolates,
    })
}

// ─── 参数表 ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Bool,
    Val,
}

fn short_to_long(c: char) -> Option<&'static str> {
    Some(match c {
        'd' => "detach",
        'i' => "interactive",
        't' => "tty",
        'P' => "publish-all",
        'q' => "quiet",
        'a' => "attach",
        'p' => "publish",
        'e' => "env",
        'v' => "volume",
        'u' => "user",
        'w' => "workdir",
        'h' => "hostname",
        'l' => "label",
        'm' => "memory",
        'c' => "cpu-shares",
        _ => return None,
    })
}

fn flag_kind(name: &str) -> Option<Kind> {
    const BOOL: &[&str] = &[
        "detach",
        "rm",
        "interactive",
        "tty",
        "privileged",
        "init",
        "read-only",
        "publish-all",
        "no-healthcheck",
        "oom-kill-disable",
        "quiet",
        "sig-proxy",
        "disable-content-trust",
    ];
    const VAL: &[&str] = &[
        "name",
        "publish",
        "expose",
        "env",
        "env-file",
        "volume",
        "mount",
        "tmpfs",
        "restart",
        "network",
        "net",
        "network-alias",
        "net-alias",
        "ip",
        "ip6",
        "hostname",
        "domainname",
        "user",
        "workdir",
        "entrypoint",
        "label",
        "cap-add",
        "cap-drop",
        "device",
        "dns",
        "dns-search",
        "dns-option",
        "add-host",
        "security-opt",
        "group-add",
        "link",
        "volumes-from",
        "shm-size",
        "memory",
        "memory-swap",
        "memory-reservation",
        "cpus",
        "cpu-shares",
        "cpuset-cpus",
        "pids-limit",
        "oom-score-adj",
        "platform",
        "pid",
        "ipc",
        "uts",
        "userns",
        "stop-signal",
        "stop-timeout",
        "mac-address",
        "runtime",
        "cgroup-parent",
        "isolation",
        "pull",
        "log-driver",
        "log-opt",
        "sysctl",
        "storage-opt",
        "ulimit",
        "health-cmd",
        "health-interval",
        "health-timeout",
        "health-start-period",
        "health-retries",
        "gpus",
        "attach",
        "cidfile",
        "detach-keys",
    ];
    if BOOL.contains(&name) {
        Some(Kind::Bool)
    } else if VAL.contains(&name) {
        Some(Kind::Val)
    } else {
        None
    }
}

// ─── Compose 构建器（两种来源共用） ──────────────────────────────────────────

#[derive(Default)]
struct ComposeBuilder {
    svc: Mapping,
    volumes: Mapping,
    networks: Mapping,
    warnings: Vec<String>,
}

fn s(v: &str) -> Value {
    Value::String(v.to_string())
}

fn int(v: i64) -> Value {
    Value::Number(Number::from(v))
}

impl ComposeBuilder {
    fn set(&mut self, key: &str, v: Value) {
        self.svc.insert(s(key), v);
    }

    fn push(&mut self, key: &str, v: Value) {
        let entry = self
            .svc
            .entry(s(key))
            .or_insert_with(|| Value::Sequence(Vec::new()));
        if let Value::Sequence(seq) = entry {
            seq.push(v);
        }
    }

    fn map_entry(&mut self, key: &str) -> &mut Mapping {
        let entry = self
            .svc
            .entry(s(key))
            .or_insert_with(|| Value::Mapping(Mapping::new()));
        if !entry.is_mapping() {
            *entry = Value::Mapping(Mapping::new());
        }
        entry.as_mapping_mut().expect("just ensured mapping")
    }

    fn put(&mut self, key: &str, k: &str, v: Value) {
        self.map_entry(key).insert(s(k), v);
    }

    fn warn(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        if !self.warnings.contains(&msg) {
            self.warnings.push(msg);
        }
    }

    /// 具名卷：用 `name:` 固定真实卷名，避免 Compose 加项目前缀后指向一个新的空卷。
    fn named_volume(&mut self, name: &str) {
        let mut m = Mapping::new();
        m.insert(s("name"), s(name));
        self.volumes.insert(s(name), Value::Mapping(m));
    }

    fn external_network(&mut self, name: &str) {
        let mut m = Mapping::new();
        m.insert(s("external"), Value::Bool(true));
        self.networks.insert(s(name), Value::Mapping(m));
    }

    /// 短语法卷（`src:dst[:opts]` / `dst`）：识别具名卷并登记。
    fn short_volume(&mut self, spec: &str) {
        let mut parts = spec.splitn(2, ':');
        let src = parts.next().unwrap_or_default();
        if parts.next().is_some() && is_named_volume(src) {
            self.named_volume(src);
        }
        self.push("volumes", s(spec));
    }

    fn finish(self, service: &str, source: &str) -> String {
        let mut services = Mapping::new();
        services.insert(s(service), Value::Mapping(self.svc));
        let mut root = Mapping::new();
        root.insert(s("services"), Value::Mapping(services));
        if !self.volumes.is_empty() {
            root.insert(s("volumes"), Value::Mapping(self.volumes));
        }
        if !self.networks.is_empty() {
            root.insert(s("networks"), Value::Mapping(self.networks));
        }
        let body = serde_yaml::to_string(&Value::Mapping(root)).unwrap_or_default();
        format!("# 由{source}转换生成（Flamepanel），部署前请检查\n{body}")
    }
}

fn is_named_volume(src: &str) -> bool {
    !src.is_empty() && !src.starts_with(['/', '.', '~', '$']) && !src.contains('/')
}

/// 服务 / 项目名：小写，非 `[a-z0-9_-]` 替换为 `-`，首字符字母数字，≤ 63。
pub fn suggest_name(raw: &str) -> String {
    let mut out: String = raw
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    out = out
        .trim_start_matches(|c: char| !c.is_ascii_alphanumeric())
        .to_string();
    out.truncate(63);
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() {
        "app".into()
    } else {
        out
    }
}

/// 镜像引用 → 默认服务名：去 digest / tag，取最后一段路径。
fn image_basename(image: &str) -> &str {
    let no_digest = image.split('@').next().unwrap_or(image);
    let last = no_digest.rsplit('/').next().unwrap_or(no_digest);
    last.split(':').next().unwrap_or(last)
}

fn parse_int(flag: &str, v: &str) -> Result<i64, AppError> {
    v.trim()
        .parse::<i64>()
        .map_err(|_| bad(format!("--{flag} 需要整数，得到 {v:?}")))
}

fn parse_bool(flag: &str, v: Option<&str>) -> Result<bool, AppError> {
    match v {
        None | Some("true") | Some("1") => Ok(true),
        Some("false") | Some("0") => Ok(false),
        Some(o) => Err(bad(format!("--{flag} 的取值应为 true / false，得到 {o:?}"))),
    }
}

fn kv(v: &str) -> (&str, Option<&str>) {
    match v.split_once('=') {
        Some((k, val)) => (k, Some(val)),
        None => (v, None),
    }
}

// ─── docker run → Compose ────────────────────────────────────────────────────

/// 把一条 `docker run` / `docker create` 命令转换为 Compose YAML。
pub fn convert_docker_run(input: &str) -> Result<ComposeConversion, AppError> {
    if input.trim().is_empty() {
        return Err(bad("命令为空"));
    }
    if input.len() > MAX_RUN_COMMAND_BYTES {
        return Err(bad(format!(
            "命令过长（上限 {} KiB）",
            MAX_RUN_COMMAND_BYTES / 1024
        )));
    }
    let Tokens {
        words,
        interpolates,
    } = tokenize(input)?;

    // 前缀：[sudo] docker|podman [container] run|create
    let mut i = 0;
    if words.first().map(String::as_str) == Some("sudo") {
        i += 1;
    }
    let prefix_ok = matches!(words.get(i).map(String::as_str), Some("docker" | "podman"));
    if !prefix_ok {
        return Err(bad("请粘贴以 docker run 开头的命令"));
    }
    i += 1;
    if words.get(i).map(String::as_str) == Some("container") {
        i += 1;
    }
    if !matches!(words.get(i).map(String::as_str), Some("run" | "create")) {
        return Err(bad("请粘贴以 docker run 开头的命令"));
    }
    i += 1;

    // 参数
    let mut opts: Vec<(String, Option<String>)> = Vec::new();
    while i < words.len() {
        let tok = &words[i];
        if tok == "--" {
            i += 1;
            break;
        }
        if let Some(rest) = tok.strip_prefix("--") {
            let (name, inline) = match rest.split_once('=') {
                Some((n, v)) => (n, Some(v.to_string())),
                None => (rest, None),
            };
            let kind = flag_kind(name)
                .ok_or_else(|| bad(format!("不支持的参数 --{name}，请转换后手动补充")))?;
            let value = match (kind, inline) {
                (Kind::Bool, v) => v,
                (Kind::Val, Some(v)) => Some(v),
                (Kind::Val, None) => {
                    i += 1;
                    Some(
                        words
                            .get(i)
                            .cloned()
                            .ok_or_else(|| bad(format!("--{name} 缺少取值")))?,
                    )
                }
            };
            opts.push((name.to_string(), value));
        } else if tok.len() > 1 && tok.starts_with('-') {
            let chars: Vec<char> = tok[1..].chars().collect();
            let mut j = 0;
            while j < chars.len() {
                let c = chars[j];
                let long = short_to_long(c)
                    .ok_or_else(|| bad(format!("不支持的参数 -{c}，请转换后手动补充")))?;
                if flag_kind(long) == Some(Kind::Bool) {
                    opts.push((long.to_string(), None));
                    j += 1;
                    continue;
                }
                let rest: String = chars[j + 1..].iter().collect();
                let value = if rest.is_empty() {
                    i += 1;
                    words
                        .get(i)
                        .cloned()
                        .ok_or_else(|| bad(format!("-{c} 缺少取值")))?
                } else {
                    rest.strip_prefix('=').unwrap_or(&rest).to_string()
                };
                opts.push((long.to_string(), Some(value)));
                break;
            }
        } else {
            break;
        }
        i += 1;
    }
    let image = words.get(i).cloned().ok_or_else(|| bad("缺少镜像名"))?;
    let command: Vec<String> = words[i + 1..].to_vec();

    let mut b = ComposeBuilder::default();
    b.set("image", s(&image));
    let mut container_name: Option<String> = None;
    let mut nets: Vec<String> = Vec::new();
    let mut aliases: Vec<String> = Vec::new();
    let mut ipv4: Option<String> = None;
    let mut ipv6: Option<String> = None;

    for (flag, value) in &opts {
        let v = value.as_deref().unwrap_or_default();
        match flag.as_str() {
            "name" => {
                container_name = Some(v.to_string());
                b.set("container_name", s(v));
            }
            "publish" => b.push("ports", s(v)),
            "expose" => b.push("expose", s(v)),
            "env" => match kv(v) {
                (k, Some(val)) => b.put("environment", k, s(val)),
                (k, None) => {
                    b.put("environment", k, Value::Null);
                    b.warn(format!(
                        "环境变量 {k} 未给出取值：Compose 将从部署环境继承，通常为空，请补充"
                    ));
                }
            },
            "env-file" => {
                b.push("env_file", s(v));
                b.warn("env_file 路径相对 Compose 项目目录解析，请确认该文件已放入项目目录");
            }
            "volume" => b.short_volume(v),
            "mount" => {
                let m = parse_mount(v, &mut b)?;
                b.push("volumes", m);
            }
            "tmpfs" => b.push("tmpfs", s(v)),
            "restart" => {
                let ok = matches!(v, "no" | "always" | "unless-stopped" | "on-failure")
                    || v.strip_prefix("on-failure:")
                        .is_some_and(|n| n.parse::<u32>().is_ok());
                if !ok {
                    return Err(bad(format!("--restart 取值无效：{v:?}")));
                }
                b.set("restart", s(v));
            }
            "network" | "net" => {
                if matches!(v, "host" | "bridge" | "none" | "default")
                    || v.starts_with("container:")
                {
                    let mode = if v == "default" { "bridge" } else { v };
                    b.set("network_mode", s(mode));
                } else {
                    nets.push(v.to_string());
                }
            }
            "network-alias" | "net-alias" => aliases.push(v.to_string()),
            "ip" => ipv4 = Some(v.to_string()),
            "ip6" => ipv6 = Some(v.to_string()),
            "hostname" => b.set("hostname", s(v)),
            "domainname" => b.set("domainname", s(v)),
            "user" => b.set("user", s(v)),
            "workdir" => b.set("working_dir", s(v)),
            "entrypoint" => {
                if v.is_empty() {
                    b.set("entrypoint", Value::Sequence(Vec::new()));
                } else {
                    b.set("entrypoint", Value::Sequence(vec![s(v)]));
                }
            }
            "label" => {
                let (k, val) = kv(v);
                b.put("labels", k, s(val.unwrap_or_default()));
            }
            "cap-add" => b.push("cap_add", s(v)),
            "cap-drop" => b.push("cap_drop", s(v)),
            "device" => b.push("devices", s(v)),
            "dns" => b.push("dns", s(v)),
            "dns-search" => b.push("dns_search", s(v)),
            "dns-option" => b.push("dns_opt", s(v)),
            "add-host" => b.push("extra_hosts", s(v)),
            "security-opt" => b.push("security_opt", s(v)),
            "group-add" => b.push("group_add", s(v)),
            "link" => {
                b.push("links", s(v));
                b.warn(
                    "--link 为旧式功能：同一 Compose 项目内服务可直接用服务名互访，建议改用网络",
                );
            }
            "volumes-from" => b.push("volumes_from", s(v)),
            "shm-size" => b.set("shm_size", s(v)),
            "memory" => b.set("mem_limit", s(v)),
            "memory-swap" => b.set("memswap_limit", s(v)),
            "memory-reservation" => b.set("mem_reservation", s(v)),
            "cpus" => {
                let n: f64 = v
                    .parse()
                    .map_err(|_| bad(format!("--cpus 需要数字，得到 {v:?}")))?;
                b.set("cpus", Value::Number(Number::from(n)));
            }
            "cpu-shares" => b.set("cpu_shares", int(parse_int(flag, v)?)),
            "cpuset-cpus" => b.set("cpuset", s(v)),
            "pids-limit" => b.set("pids_limit", int(parse_int(flag, v)?)),
            "oom-score-adj" => b.set("oom_score_adj", int(parse_int(flag, v)?)),
            "platform" => b.set("platform", s(v)),
            "pid" => b.set("pid", s(v)),
            "ipc" => b.set("ipc", s(v)),
            "uts" => b.set("uts", s(v)),
            "userns" => b.set("userns_mode", s(v)),
            "stop-signal" => b.set("stop_signal", s(v)),
            "stop-timeout" => b.set("stop_grace_period", s(&format!("{}s", parse_int(flag, v)?))),
            "mac-address" => b.set("mac_address", s(v)),
            "runtime" => b.set("runtime", s(v)),
            "cgroup-parent" => b.set("cgroup_parent", s(v)),
            "isolation" => b.set("isolation", s(v)),
            "pull" => {
                if !matches!(v, "always" | "missing" | "never") {
                    return Err(bad(format!("--pull 取值无效：{v:?}")));
                }
                b.set("pull_policy", s(v));
            }
            "log-driver" => {
                b.map_entry("logging").insert(s("driver"), s(v));
            }
            "log-opt" => {
                let (k, val) = kv(v);
                let logging = b.map_entry("logging");
                let opts = logging
                    .entry(s("options"))
                    .or_insert_with(|| Value::Mapping(Mapping::new()));
                if let Value::Mapping(m) = opts {
                    m.insert(s(k), s(val.unwrap_or_default()));
                }
            }
            "sysctl" => {
                let (k, val) = kv(v);
                b.put("sysctls", k, s(val.unwrap_or_default()));
            }
            "storage-opt" => {
                let (k, val) = kv(v);
                b.put("storage_opt", k, s(val.unwrap_or_default()));
            }
            "ulimit" => {
                let (k, val) = kv(v);
                let val = val.ok_or_else(|| {
                    bad(format!("--ulimit 格式应为 name=soft[:hard]，得到 {v:?}"))
                })?;
                let limit = match val.split_once(':') {
                    Some((soft, hard)) => {
                        let mut m = Mapping::new();
                        m.insert(s("soft"), int(parse_int(flag, soft)?));
                        m.insert(s("hard"), int(parse_int(flag, hard)?));
                        Value::Mapping(m)
                    }
                    None => int(parse_int(flag, val)?),
                };
                b.put("ulimits", k, limit);
            }
            "health-cmd" => b.put(
                "healthcheck",
                "test",
                Value::Sequence(vec![s("CMD-SHELL"), s(v)]),
            ),
            "health-interval" => b.put("healthcheck", "interval", s(v)),
            "health-timeout" => b.put("healthcheck", "timeout", s(v)),
            "health-start-period" => b.put("healthcheck", "start_period", s(v)),
            "health-retries" => b.put("healthcheck", "retries", int(parse_int(flag, v)?)),
            "no-healthcheck" => {
                if parse_bool(flag, value.as_deref())? {
                    b.put("healthcheck", "disable", Value::Bool(true));
                }
            }
            "gpus" => gpus(v, &mut b),
            "privileged" | "init" | "read-only" | "interactive" | "tty" | "oom-kill-disable" => {
                let key = match flag.as_str() {
                    "read-only" => "read_only",
                    "interactive" => "stdin_open",
                    "oom-kill-disable" => "oom_kill_disable",
                    other => other,
                };
                if parse_bool(flag, value.as_deref())? {
                    b.set(key, Value::Bool(true));
                }
            }
            "rm" => b.warn("--rm 在 Compose 中无对应项，已忽略（down 时会删除容器）"),
            "publish-all" => {
                b.warn("-P / --publish-all 的随机端口无法等价转换，已忽略；请改用 -p 显式映射")
            }
            // 与运行方式相关、对 Compose 无意义的参数
            "detach"
            | "quiet"
            | "sig-proxy"
            | "attach"
            | "cidfile"
            | "detach-keys"
            | "disable-content-trust" => {}
            other => return Err(bad(format!("不支持的参数 --{other}"))),
        }
    }

    // 网络：自定义网络须已存在（docker run 同样要求），声明为 external
    if nets.is_empty() {
        if !aliases.is_empty() || ipv4.is_some() || ipv6.is_some() {
            b.warn("--network-alias / --ip 需要配合自定义网络，已忽略");
        }
    } else {
        let per_net = !aliases.is_empty() || ipv4.is_some() || ipv6.is_some();
        if per_net {
            let mut nm = Mapping::new();
            for (idx, n) in nets.iter().enumerate() {
                let mut cfg = Mapping::new();
                if idx == 0 {
                    if !aliases.is_empty() {
                        cfg.insert(
                            s("aliases"),
                            Value::Sequence(aliases.iter().map(|a| s(a)).collect()),
                        );
                    }
                    if let Some(ip) = &ipv4 {
                        cfg.insert(s("ipv4_address"), s(ip));
                    }
                    if let Some(ip) = &ipv6 {
                        cfg.insert(s("ipv6_address"), s(ip));
                    }
                }
                nm.insert(
                    s(n),
                    if cfg.is_empty() {
                        Value::Null
                    } else {
                        Value::Mapping(cfg)
                    },
                );
            }
            b.set("networks", Value::Mapping(nm));
        } else {
            b.set(
                "networks",
                Value::Sequence(nets.iter().map(|n| s(n)).collect()),
            );
        }
        for n in &nets {
            b.external_network(n);
        }
    }

    if !command.is_empty() {
        b.set(
            "command",
            Value::Sequence(command.iter().map(|c| s(c)).collect()),
        );
    }
    if interpolates {
        b.warn("命令中含 $ 变量：Compose 会在部署时按面板进程环境插值（未定义则为空），请替换为实际值；字面 $ 请写成 $$");
    }

    let service = suggest_name(
        container_name
            .as_deref()
            .unwrap_or_else(|| image_basename(&image)),
    );
    let warnings = std::mem::take(&mut b.warnings);
    let yaml = b.finish(&service, " docker run 命令");
    Ok(ComposeConversion {
        project_name: service.clone(),
        service_name: service,
        compose_yaml: yaml,
        warnings,
    })
}

fn parse_mount(spec: &str, b: &mut ComposeBuilder) -> Result<Value, AppError> {
    let mut kind = "volume".to_string();
    let mut source: Option<String> = None;
    let mut target: Option<String> = None;
    let mut read_only = false;
    let mut extra: Vec<(String, Value)> = Vec::new();
    for part in spec.split(',').filter(|p| !p.is_empty()) {
        let (k, v) = kv(part);
        match k {
            "type" => kind = v.unwrap_or_default().to_string(),
            "source" | "src" => source = v.map(str::to_string),
            "target" | "destination" | "dst" => target = v.map(str::to_string),
            "readonly" | "ro" => read_only = parse_bool("mount readonly", v)?,
            "bind-propagation" => {
                let mut m = Mapping::new();
                m.insert(s("propagation"), s(v.unwrap_or_default()));
                extra.push(("bind".into(), Value::Mapping(m)));
            }
            "volume-nocopy" => {
                let mut m = Mapping::new();
                m.insert(
                    s("nocopy"),
                    Value::Bool(parse_bool("mount volume-nocopy", v)?),
                );
                extra.push(("volume".into(), Value::Mapping(m)));
            }
            "tmpfs-size" => {
                let mut m = Mapping::new();
                m.insert(s("size"), s(v.unwrap_or_default()));
                extra.push(("tmpfs".into(), Value::Mapping(m)));
            }
            other => b.warn(format!("--mount 选项 {other} 未转换，请手动补充")),
        }
    }
    let target = target.ok_or_else(|| bad(format!("--mount 缺少 target：{spec:?}")))?;
    if !matches!(kind.as_str(), "bind" | "volume" | "tmpfs") {
        return Err(bad(format!(
            "--mount type 仅支持 bind / volume / tmpfs，得到 {kind:?}"
        )));
    }
    if kind == "volume" {
        if let Some(src) = &source {
            b.named_volume(src);
        }
    }
    let mut m = Mapping::new();
    m.insert(s("type"), s(&kind));
    if let Some(src) = &source {
        m.insert(s("source"), s(src));
    }
    m.insert(s("target"), s(&target));
    if read_only {
        m.insert(s("read_only"), Value::Bool(true));
    }
    for (k, v) in extra {
        m.insert(s(&k), v);
    }
    Ok(Value::Mapping(m))
}

fn gpus(v: &str, b: &mut ComposeBuilder) {
    let mut dev = Mapping::new();
    dev.insert(s("driver"), s("nvidia"));
    let v = v.trim_matches('"');
    if v == "all" {
        dev.insert(s("count"), s("all"));
    } else if let Ok(n) = v.parse::<i64>() {
        dev.insert(s("count"), int(n));
    } else if let Some(ids) = v.strip_prefix("device=") {
        dev.insert(
            s("device_ids"),
            Value::Sequence(ids.split(',').map(s).collect()),
        );
    } else {
        b.warn(format!(
            "--gpus {v:?} 未能转换，请手动配置 deploy.resources"
        ));
        return;
    }
    dev.insert(s("capabilities"), Value::Sequence(vec![s("gpu")]));
    let mut reservations = Mapping::new();
    reservations.insert(s("devices"), Value::Sequence(vec![Value::Mapping(dev)]));
    let mut resources = Mapping::new();
    resources.insert(s("reservations"), Value::Mapping(reservations));
    let mut deploy = Mapping::new();
    deploy.insert(s("resources"), Value::Mapping(resources));
    b.set("deploy", Value::Mapping(deploy));
}

// ─── 既有容器 → Compose（收编） ──────────────────────────────────────────────

fn str_list(v: &Json) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn jstr<'a>(v: &'a Json, ptr: &str) -> &'a str {
    v.pointer(ptr).and_then(Json::as_str).unwrap_or_default()
}

fn jbool(v: &Json, ptr: &str) -> bool {
    v.pointer(ptr).and_then(Json::as_bool).unwrap_or(false)
}

fn ji64(v: &Json, ptr: &str) -> i64 {
    v.pointer(ptr).and_then(Json::as_i64).unwrap_or(0)
}

/// 纳秒 → Compose 时长字符串（`30s` / `500ms`）。
fn duration_ns(ns: i64) -> String {
    if ns % 1_000_000_000 == 0 {
        format!("{}s", ns / 1_000_000_000)
    } else {
        format!("{}ms", ns / 1_000_000)
    }
}

/// 把 `docker inspect` 结果转换为等价 Compose。
///
/// `image` 为同一镜像的 `docker image inspect`：用于剔除镜像自带的环境变量 / 命令 /
/// 标签等默认值（它们在容器 inspect 里与用户显式设置的混在一起），缺省时全部保留并提示。
pub fn convert_container(
    container: &Json,
    image: Option<&Json>,
) -> Result<ComposeConversion, AppError> {
    let image_ref = jstr(container, "/Config/Image");
    if image_ref.is_empty() {
        return Err(bad("容器信息缺少镜像（Config.Image）"));
    }
    let null = Json::Null;
    let img = image.filter(|v| v.is_object()).unwrap_or(&null);
    let mut b = ComposeBuilder::default();
    if img.is_null() {
        b.warn("未能读取镜像默认配置：环境变量 / 命令 / 标签可能包含镜像自带的默认值，请按需删减");
    }
    b.set("image", s(image_ref));
    let name = jstr(container, "/Name").trim_start_matches('/').to_string();
    if !name.is_empty() {
        b.set("container_name", s(&name));
    }

    let labels = container
        .pointer("/Config/Labels")
        .and_then(Json::as_object);
    if let Some(project) = labels
        .and_then(|l| l.get("com.docker.compose.project"))
        .and_then(Json::as_str)
    {
        b.warn(format!(
            "该容器已属于 Compose 项目 {project}：建议直接在 Compose 页签管理，而不是重复收编"
        ));
    }

    // 命令 / 入口 / 工作目录 / 用户：与镜像默认不同才写
    for (ptr, key) in [
        ("/Config/Entrypoint", "entrypoint"),
        ("/Config/Cmd", "command"),
    ] {
        let cur = container.pointer(ptr).cloned().unwrap_or(Json::Null);
        if !cur.is_null() && img.pointer(ptr) != Some(&cur) {
            let list = str_list(&cur);
            if !list.is_empty() {
                b.set(key, Value::Sequence(list.iter().map(|x| s(x)).collect()));
            }
        }
    }
    for (ptr, key) in [
        ("/Config/WorkingDir", "working_dir"),
        ("/Config/User", "user"),
    ] {
        let cur = jstr(container, ptr);
        if !cur.is_empty() && cur != jstr(img, ptr) {
            b.set(key, s(cur));
        }
    }
    let id = jstr(container, "/Id");
    let hostname = jstr(container, "/Config/Hostname");
    if !hostname.is_empty() && !id.starts_with(hostname) {
        b.set("hostname", s(hostname));
    }

    // 环境变量：剔除镜像自带的
    let image_env = str_list(img.pointer("/Config/Env").unwrap_or(&null));
    for e in str_list(container.pointer("/Config/Env").unwrap_or(&null)) {
        if image_env.contains(&e) {
            continue;
        }
        let (k, v) = kv(&e);
        b.put("environment", k, s(v.unwrap_or_default()));
    }

    // 标签：剔除镜像自带与 Compose / OCI 元数据
    if let Some(labels) = labels {
        let image_labels = img.pointer("/Config/Labels").and_then(Json::as_object);
        for (k, v) in labels {
            let from_image = image_labels.and_then(|l| l.get(k)) == Some(v);
            if from_image
                || k.starts_with("com.docker.compose.")
                || k.starts_with("org.opencontainers.image.")
            {
                continue;
            }
            b.put("labels", k, s(v.as_str().unwrap_or_default()));
        }
    }

    // 端口
    if let Some(bindings) = container
        .pointer("/HostConfig/PortBindings")
        .and_then(Json::as_object)
    {
        let mut ports: Vec<String> = Vec::new();
        for (cport, hosts) in bindings {
            let cport_s = cport.strip_suffix("/tcp").unwrap_or(cport);
            let list = hosts.as_array().cloned().unwrap_or_default();
            if list.is_empty() {
                ports.push(cport_s.to_string());
            }
            for h in list {
                let ip = jstr(&h, "/HostIp");
                let hp = jstr(&h, "/HostPort");
                let spec = match (ip, hp) {
                    (_, "") => cport_s.to_string(),
                    ("" | "0.0.0.0", hp) => format!("{hp}:{cport_s}"),
                    (ip, hp) if ip.contains(':') => format!("[{ip}]:{hp}:{cport_s}"),
                    (ip, hp) => format!("{ip}:{hp}:{cport_s}"),
                };
                if !ports.contains(&spec) {
                    ports.push(spec);
                }
            }
        }
        ports.sort();
        for p in ports {
            b.push("ports", s(&p));
        }
    }

    // 挂载
    for m in container
        .pointer("/Mounts")
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let dst = jstr(&m, "/Destination");
        let ro = m.pointer("/RW").and_then(Json::as_bool) == Some(false);
        let suffix = if ro { ":ro" } else { "" };
        match jstr(&m, "/Type") {
            "bind" => b.push(
                "volumes",
                s(&format!("{}:{dst}{suffix}", jstr(&m, "/Source"))),
            ),
            "volume" => {
                let vname = jstr(&m, "/Name");
                let anonymous = vname.len() == 64 && vname.bytes().all(|c| c.is_ascii_hexdigit());
                if vname.is_empty() || anonymous {
                    b.push("volumes", s(dst));
                    b.warn(format!(
                        "{dst} 是匿名卷：重新部署后会得到一个新的空卷，如需保留数据请改为具名卷或绑定目录"
                    ));
                } else {
                    b.named_volume(vname);
                    b.push("volumes", s(&format!("{vname}:{dst}{suffix}")));
                }
            }
            "tmpfs" => b.push("tmpfs", s(dst)),
            other => b.warn(format!("挂载类型 {other}（{dst}）未转换，请手动补充")),
        }
    }

    // 重启策略
    let restart = jstr(container, "/HostConfig/RestartPolicy/Name");
    match restart {
        "" | "no" => {}
        "on-failure" => {
            let n = ji64(container, "/HostConfig/RestartPolicy/MaximumRetryCount");
            let v = if n > 0 {
                format!("on-failure:{n}")
            } else {
                "on-failure".into()
            };
            b.set("restart", s(&v));
        }
        other => b.set("restart", s(other)),
    }

    // 网络
    let mode = jstr(container, "/HostConfig/NetworkMode");
    match mode {
        "" => {}
        "default" | "bridge" => b.set("network_mode", s("bridge")),
        "host" | "none" => b.set("network_mode", s(mode)),
        m if m.starts_with("container:") => b.set("network_mode", s(m)),
        net => {
            b.set("networks", Value::Sequence(vec![s(net)]));
            b.external_network(net);
        }
    }

    // 安全与资源
    if jbool(container, "/HostConfig/Privileged") {
        b.set("privileged", Value::Bool(true));
    }
    for (ptr, key) in [
        ("/HostConfig/CapAdd", "cap_add"),
        ("/HostConfig/CapDrop", "cap_drop"),
        ("/HostConfig/Dns", "dns"),
        ("/HostConfig/ExtraHosts", "extra_hosts"),
        ("/HostConfig/SecurityOpt", "security_opt"),
    ] {
        for x in str_list(container.pointer(ptr).unwrap_or(&null)) {
            b.push(key, s(&x));
        }
    }
    for d in container
        .pointer("/HostConfig/Devices")
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let host = jstr(&d, "/PathOnHost");
        let inner = jstr(&d, "/PathInContainer");
        let spec = if inner.is_empty() || inner == host {
            host.to_string()
        } else {
            format!("{host}:{inner}")
        };
        b.push("devices", s(&spec));
    }
    let shm = ji64(container, "/HostConfig/ShmSize");
    if shm > 0 && shm != 64 * 1024 * 1024 {
        b.set("shm_size", int(shm));
    }
    let mem = ji64(container, "/HostConfig/Memory");
    if mem > 0 {
        b.set("mem_limit", int(mem));
    }
    let nano = ji64(container, "/HostConfig/NanoCpus");
    if nano > 0 {
        b.set("cpus", Value::Number(Number::from(nano as f64 / 1e9)));
    }
    if jbool(container, "/HostConfig/Init") {
        b.set("init", Value::Bool(true));
    }
    if jbool(container, "/HostConfig/ReadonlyRootfs") {
        b.set("read_only", Value::Bool(true));
    }
    if jbool(container, "/Config/Tty") {
        b.set("tty", Value::Bool(true));
    }
    if jbool(container, "/Config/OpenStdin") {
        b.set("stdin_open", Value::Bool(true));
    }

    // 日志
    let log_type = jstr(container, "/HostConfig/LogConfig/Type");
    let log_cfg = container
        .pointer("/HostConfig/LogConfig/Config")
        .and_then(Json::as_object)
        .filter(|m| !m.is_empty());
    if (!log_type.is_empty() && log_type != "json-file") || log_cfg.is_some() {
        let logging = b.map_entry("logging");
        if !log_type.is_empty() {
            logging.insert(s("driver"), s(log_type));
        }
        if let Some(cfg) = log_cfg {
            let mut m = Mapping::new();
            for (k, v) in cfg {
                m.insert(s(k), s(v.as_str().unwrap_or_default()));
            }
            logging.insert(s("options"), Value::Mapping(m));
        }
    }

    // 健康检查：与镜像默认不同才写
    if let Some(hc) = container
        .pointer("/Config/Healthcheck")
        .filter(|h| h.is_object())
    {
        if img.pointer("/Config/Healthcheck") != Some(hc) {
            let test = str_list(hc.pointer("/Test").unwrap_or(&null));
            if test.first().map(String::as_str) == Some("NONE") {
                b.put("healthcheck", "disable", Value::Bool(true));
            } else if !test.is_empty() {
                b.put(
                    "healthcheck",
                    "test",
                    Value::Sequence(test.iter().map(|x| s(x)).collect()),
                );
                for (ptr, key) in [
                    ("/Interval", "interval"),
                    ("/Timeout", "timeout"),
                    ("/StartPeriod", "start_period"),
                ] {
                    let ns = ji64(hc, ptr);
                    if ns > 0 {
                        b.put("healthcheck", key, s(&duration_ns(ns)));
                    }
                }
                let retries = ji64(hc, "/Retries");
                if retries > 0 {
                    b.put("healthcheck", "retries", int(retries));
                }
            }
        }
    }

    let service = suggest_name(if name.is_empty() {
        image_basename(image_ref)
    } else {
        &name
    });
    b.warn("收编步骤：确认 YAML 无误 → 停止并删除原容器（具名卷与绑定目录的数据会保留）→ 以此 YAML 部署");
    let warnings = std::mem::take(&mut b.warnings);
    let yaml = b.finish(&service, "既有容器");
    Ok(ComposeConversion {
        project_name: service.clone(),
        service_name: service,
        compose_yaml: yaml,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::docker_ref::validate_project_name;
    use serde_json::json;

    fn svc(conv: &ComposeConversion) -> Value {
        let doc: Value = serde_yaml::from_str(&conv.compose_yaml).expect("valid yaml");
        doc["services"][conv.service_name.as_str()].clone()
    }

    fn run(cmd: &str) -> ComposeConversion {
        let c = convert_docker_run(cmd).unwrap_or_else(|e| panic!("{cmd}: {e:?}"));
        validate_project_name(&c.project_name).unwrap();
        c
    }

    fn rejects(cmd: &str) -> String {
        match convert_docker_run(cmd) {
            Err(AppError::ValidationError(m)) => m,
            other => panic!("{cmd} should be rejected, got {other:?}"),
        }
    }

    #[test]
    fn converts_common_flags() {
        let c = run(
            "docker run -d --name my-nginx -p 8080:80 -p 127.0.0.1:8443:443/tcp \
             -v /srv/www:/usr/share/nginx/html:ro -e TZ=Asia/Shanghai -e DEBUG \
             --restart unless-stopped --memory 512m --cpus 1.5 nginx:alpine",
        );
        assert_eq!(c.service_name, "my-nginx");
        let s = svc(&c);
        assert_eq!(s["image"], "nginx:alpine");
        assert_eq!(s["container_name"], "my-nginx");
        assert_eq!(s["ports"][0], "8080:80");
        assert_eq!(s["ports"][1], "127.0.0.1:8443:443/tcp");
        assert_eq!(s["volumes"][0], "/srv/www:/usr/share/nginx/html:ro");
        assert_eq!(s["environment"]["TZ"], "Asia/Shanghai");
        assert!(s["environment"]["DEBUG"].is_null());
        assert_eq!(s["restart"], "unless-stopped");
        assert_eq!(s["mem_limit"], "512m");
        assert_eq!(s["cpus"].as_f64(), Some(1.5));
        assert!(c.warnings.iter().any(|w| w.contains("DEBUG")));
        assert!(c.compose_yaml.starts_with("# 由 docker run 命令转换生成"));
    }

    #[test]
    fn follows_shell_quoting_and_continuations() {
        let c = run("sudo docker container run \\\n  -e 'PASS=a b$c' \\\n  -e \"Q=\\\"x\\\"\" \\\r\n  -e X=\\$HOME \
             alpine sh -c 'echo hi; exit 0'  # 注释");
        let s = svc(&c);
        assert_eq!(s["environment"]["PASS"], "a b$$c", "单引号内的 $ 为字面量");
        assert_eq!(s["environment"]["Q"], "\"x\"");
        assert_eq!(s["environment"]["X"], "$$HOME");
        assert_eq!(
            s["command"],
            serde_yaml::from_str::<Value>("[sh, -c, 'echo hi; exit 0']").unwrap()
        );
        assert_eq!(c.service_name, "alpine");
        assert!(!c.warnings.iter().any(|w| w.contains("$ 变量")));
    }

    #[test]
    fn warns_about_interpolated_variables() {
        let c = run("docker run -e HOME=$HOME -e \"U=${USER}\" busybox");
        assert_eq!(svc(&c)["environment"]["HOME"], "$HOME");
        assert!(c.warnings.iter().any(|w| w.contains("$ 变量")));
    }

    #[test]
    fn short_flag_clusters_and_ignored_flags() {
        let c = run("docker run -dit --rm -p80:80 -eA=1 -P redis:7 redis-server --appendonly yes");
        let s = svc(&c);
        assert_eq!(s["stdin_open"], true);
        assert_eq!(s["tty"], true);
        assert_eq!(s["ports"][0], "80:80");
        assert_eq!(s["environment"]["A"], "1");
        assert_eq!(
            s["command"],
            serde_yaml::from_str::<Value>("[redis-server, --appendonly, 'yes']").unwrap()
        );
        assert!(c.warnings.iter().any(|w| w.contains("--rm")));
        assert!(c.warnings.iter().any(|w| w.contains("--publish-all")));
        assert_eq!(c.service_name, "redis");
    }

    #[test]
    fn named_volumes_keep_their_real_name() {
        let c = run("docker run -v pgdata:/var/lib/postgresql/data -v ./conf:/etc/x -v /anon \
             --mount type=volume,src=cache,dst=/cache,ro --mount type=bind,source=/opt,target=/opt,bind-propagation=rshared \
             --tmpfs /run postgres:16");
        let doc: Value = serde_yaml::from_str(&c.compose_yaml).unwrap();
        assert_eq!(doc["volumes"]["pgdata"]["name"], "pgdata");
        assert_eq!(doc["volumes"]["cache"]["name"], "cache");
        assert!(doc["volumes"].get("./conf").is_none());
        let s = svc(&c);
        assert_eq!(s["volumes"][3]["type"], "volume");
        assert_eq!(s["volumes"][3]["read_only"], true);
        assert_eq!(s["volumes"][4]["bind"]["propagation"], "rshared");
        assert_eq!(s["tmpfs"][0], "/run");
    }

    #[test]
    fn networks_are_external_and_carry_aliases() {
        let c = run("docker run --network backend --network-alias api --ip 172.20.0.5 app:1");
        let doc: Value = serde_yaml::from_str(&c.compose_yaml).unwrap();
        assert_eq!(doc["networks"]["backend"]["external"], true);
        let s = svc(&c);
        assert_eq!(s["networks"]["backend"]["aliases"][0], "api");
        assert_eq!(s["networks"]["backend"]["ipv4_address"], "172.20.0.5");

        let host = run("docker run --net=host app:1");
        assert_eq!(svc(&host)["network_mode"], "host");
        let plain = run("docker run --network-alias x app:1");
        assert!(plain.warnings.iter().any(|w| w.contains("自定义网络")));
    }

    #[test]
    fn advanced_options() {
        let c = run("docker run --ulimit nofile=1024:4096 --ulimit nproc=512 --gpus all \
             --health-cmd 'curl -f http://localhost/ || exit 1' --health-interval 30s --health-retries 3 \
             --log-driver json-file --log-opt max-size=10m --log-opt max-file=3 \
             --cap-add NET_ADMIN --device /dev/fuse --add-host host.docker.internal:host-gateway \
             --sysctl net.core.somaxconn=1024 --stop-timeout 20 --entrypoint /bin/sh --privileged \
             --pull=always ghcr.io/org/tool:2.0@sha256:abcd -c true");
        let s = svc(&c);
        assert_eq!(c.service_name, "tool");
        assert_eq!(s["ulimits"]["nofile"]["soft"], 1024);
        assert_eq!(s["ulimits"]["nofile"]["hard"], 4096);
        assert_eq!(s["ulimits"]["nproc"], 512);
        assert_eq!(
            s["deploy"]["resources"]["reservations"]["devices"][0]["count"],
            "all"
        );
        assert_eq!(s["healthcheck"]["test"][0], "CMD-SHELL");
        assert_eq!(
            s["healthcheck"]["test"][1],
            "curl -f http://localhost/ || exit 1"
        );
        assert_eq!(s["healthcheck"]["retries"], 3);
        assert_eq!(s["logging"]["options"]["max-size"], "10m");
        assert_eq!(s["logging"]["driver"], "json-file");
        assert_eq!(s["cap_add"][0], "NET_ADMIN");
        assert_eq!(s["extra_hosts"][0], "host.docker.internal:host-gateway");
        assert_eq!(s["sysctls"]["net.core.somaxconn"], "1024");
        assert_eq!(s["stop_grace_period"], "20s");
        assert_eq!(s["entrypoint"][0], "/bin/sh");
        assert_eq!(s["privileged"], true);
        assert_eq!(s["pull_policy"], "always");
    }

    #[test]
    fn rejects_non_run_input_and_shell_constructs() {
        assert!(rejects("").contains("为空"));
        assert!(rejects("ls -la").contains("docker run"));
        assert!(rejects("docker ps").contains("docker run"));
        assert!(rejects("docker run nginx | tee x").contains("单条"));
        assert!(rejects("docker run nginx && rm -rf /").contains("单条"));
        assert!(rejects("docker run -e X=$(id) nginx").contains("单条"));
        assert!(rejects("docker run -e \"X=`id`\" nginx").contains("单条"));
        assert!(rejects("docker run nginx > out").contains("单条"));
        assert!(rejects("docker run --foo bar nginx").contains("--foo"));
        assert!(rejects("docker run -Z nginx").contains("-Z"));
        assert!(rejects("docker run -d").contains("镜像"));
        assert!(rejects("docker run --name").contains("缺少取值"));
        assert!(rejects("docker run -e 'X=1 nginx").contains("单引号"));
        assert!(rejects("docker run --restart sometimes nginx").contains("--restart"));
        assert!(rejects("docker run --cpus lots nginx").contains("--cpus"));
        assert!(rejects("docker run --mount type=bind,src=/a nginx").contains("target"));
        let long = format!(
            "docker run -e X={} nginx",
            "a".repeat(MAX_RUN_COMMAND_BYTES)
        );
        assert!(rejects(&long).contains("过长"));
    }

    #[test]
    fn yaml_values_cannot_break_structure() {
        let c = run("docker run -e 'EVIL=x\nservices:\n  pwn: {image: evil}' -l 'a=b: c' alpine");
        let doc: Value = serde_yaml::from_str(&c.compose_yaml).unwrap();
        assert_eq!(doc["services"].as_mapping().unwrap().len(), 1);
        assert_eq!(
            doc["services"]["alpine"]["environment"]["EVIL"],
            "x\nservices:\n  pwn: {image: evil}"
        );
        assert_eq!(doc["services"]["alpine"]["labels"]["a"], "b: c");
    }

    #[test]
    fn suggests_valid_names() {
        assert_eq!(suggest_name("My_App.v2"), "my_app-v2");
        assert_eq!(suggest_name("--x"), "x");
        assert_eq!(suggest_name("..."), "app");
        assert_eq!(suggest_name(&"a".repeat(80)).len(), 63);
        assert_eq!(image_basename("ghcr.io/org/app:1.0@sha256:ff"), "app");
        assert_eq!(image_basename("localhost:5000/team/web"), "web");
        let c = run("docker run --name 'Web Server!' nginx");
        assert_eq!(c.project_name, "web-server");
    }

    fn inspect() -> (Json, Json) {
        let image = json!({
            "Config": {
                "Env": ["PATH=/usr/bin", "NGINX_VERSION=1.27"],
                "Cmd": ["nginx", "-g", "daemon off;"],
                "Entrypoint": ["/docker-entrypoint.sh"],
                "Labels": { "maintainer": "NGINX" },
                "WorkingDir": ""
            }
        });
        let container = json!({
            "Id": "0123456789abcdef0123",
            "Name": "/web",
            "Config": {
                "Image": "nginx:1.27",
                "Hostname": "0123456789ab",
                "Env": ["PATH=/usr/bin", "NGINX_VERSION=1.27", "TZ=UTC"],
                "Cmd": ["nginx", "-g", "daemon off;"],
                "Entrypoint": ["/docker-entrypoint.sh"],
                "Labels": {
                    "maintainer": "NGINX",
                    "com.docker.compose.project": "old",
                    "team": "ops"
                },
                "Healthcheck": {
                    "Test": ["CMD", "curl", "-f", "http://localhost"],
                    "Interval": 30000000000i64,
                    "Timeout": 500000000,
                    "Retries": 3
                }
            },
            "HostConfig": {
                "PortBindings": {
                    "80/tcp": [{ "HostIp": "", "HostPort": "8080" }],
                    "53/udp": [{ "HostIp": "127.0.0.1", "HostPort": "5353" }]
                },
                "RestartPolicy": { "Name": "on-failure", "MaximumRetryCount": 3 },
                "NetworkMode": "backend",
                "CapAdd": ["NET_ADMIN"],
                "ShmSize": 67108864,
                "Memory": 268435456,
                "NanoCpus": 1500000000,
                "LogConfig": { "Type": "json-file", "Config": { "max-size": "10m" } }
            },
            "Mounts": [
                { "Type": "bind", "Source": "/srv/www", "Destination": "/usr/share/nginx/html", "RW": false },
                { "Type": "volume", "Name": "webdata", "Destination": "/data", "RW": true },
                { "Type": "volume", "Name": "a".repeat(64), "Destination": "/cache", "RW": true }
            ]
        });
        (container, image)
    }

    #[test]
    fn converts_container_without_image_defaults() {
        let (container, image) = inspect();
        let c = convert_container(&container, Some(&image)).unwrap();
        validate_project_name(&c.project_name).unwrap();
        assert_eq!(c.service_name, "web");
        let doc: Value = serde_yaml::from_str(&c.compose_yaml).unwrap();
        let s = &doc["services"]["web"];
        assert_eq!(s["image"], "nginx:1.27");
        assert_eq!(s["environment"].as_mapping().unwrap().len(), 1, "只保留 TZ");
        assert_eq!(s["environment"]["TZ"], "UTC");
        assert!(s.get("command").is_none() && s.get("entrypoint").is_none());
        assert!(s.get("hostname").is_none(), "默认主机名（ID 前缀）不写");
        assert_eq!(s["labels"].as_mapping().unwrap().len(), 1);
        assert_eq!(s["labels"]["team"], "ops");
        assert_eq!(s["ports"][0], "127.0.0.1:5353:53/udp");
        assert_eq!(s["ports"][1], "8080:80");
        assert_eq!(s["volumes"][0], "/srv/www:/usr/share/nginx/html:ro");
        assert_eq!(s["volumes"][1], "webdata:/data");
        assert_eq!(s["volumes"][2], "/cache");
        assert_eq!(doc["volumes"]["webdata"]["name"], "webdata");
        assert_eq!(s["restart"], "on-failure:3");
        assert_eq!(s["networks"][0], "backend");
        assert_eq!(doc["networks"]["backend"]["external"], true);
        assert!(s.get("shm_size").is_none(), "默认 64M 不写");
        assert_eq!(s["mem_limit"], 268435456);
        assert_eq!(s["cpus"].as_f64(), Some(1.5));
        assert_eq!(s["logging"]["options"]["max-size"], "10m");
        assert_eq!(s["healthcheck"]["interval"], "30s");
        assert_eq!(s["healthcheck"]["timeout"], "500ms");
        assert!(c.warnings.iter().any(|w| w.contains("Compose 项目 old")));
        assert!(c.warnings.iter().any(|w| w.contains("匿名卷")));
        assert!(c.warnings.iter().any(|w| w.contains("收编步骤")));
    }

    #[test]
    fn container_without_image_info_keeps_everything_and_warns() {
        let (container, _) = inspect();
        let c = convert_container(&container, None).unwrap();
        let doc: Value = serde_yaml::from_str(&c.compose_yaml).unwrap();
        assert_eq!(
            doc["services"]["web"]["environment"]
                .as_mapping()
                .unwrap()
                .len(),
            3
        );
        assert!(doc["services"]["web"].get("command").is_some());
        assert!(c.warnings.iter().any(|w| w.contains("镜像默认配置")));
        assert!(convert_container(&json!({ "Config": {} }), None).is_err());
    }
}
