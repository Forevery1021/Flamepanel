//! 插件市场原型（Doc/21 §5.8，Doc/30 M-D）。
//!
//! 两类来源：
//! - **内置目录**：随面板发布的示例插件（WAT 源码内嵌，安装时编译），演示宿主 API 与能力授权；
//! - **远程索引**（可选）：设置项 `plugin_market_url` 指向的 JSON 索引
//!   `{"plugins":[{id,name,version,author,description,capabilities,tags,homepage,license,wasm_url,wasm_sha256}]}`。
//!   索引 ≤ 1 MiB、包体 ≤ 8 MiB，**包体 SHA-256 必须与索引一致**才安装。
//!
//! 安装 = 管理员在 UI 中查看插件申请的能力并显式同意后，按与 `POST /api/plugins` 相同的
//! 流程加载（能力校验 → 沙箱加载 → 注册 → 落库），见 `api::handler::plugin::market`。

use crate::core::error::AppError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// 远程索引大小上限
pub const MAX_INDEX_BYTES: usize = 1024 * 1024;
/// 远程包体大小上限
pub const MAX_WASM_BYTES: usize = 8 * 1024 * 1024;
/// 远程市场索引 URL 的设置键
pub const MARKET_URL_SETTING: &str = "plugin_market_url";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MarketSource {
    Builtin,
    Remote,
}

/// 市场条目（列表展示 + 安装依据）。
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MarketPlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    /// 插件申请的宿主能力（安装时须管理员同意）
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    /// 推荐调用的导出函数
    #[serde(default = "default_entry")]
    pub entry: String,
    /// 远程包体地址（内置条目为空）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wasm_url: Option<String>,
    /// 包体 SHA-256（远程条目必填；内置条目为编译后的哈希）
    #[serde(default)]
    pub wasm_sha256: String,
    #[serde(default = "default_source")]
    pub source: MarketSource,
}

fn default_entry() -> String {
    "run".into()
}
fn default_source() -> MarketSource {
    MarketSource::Remote
}

#[derive(Deserialize)]
struct RemoteIndex {
    plugins: Vec<MarketPlugin>,
}

/// 内置插件：hello —— 无能力；读取输入并输出问候语，演示 log / input / output。
pub const HELLO_WAT: &str = r#"(module
  (import "flame" "log" (func $log (param i32 i32 i32)))
  (import "flame" "input_len" (func $input_len (result i32)))
  (import "flame" "input_read" (func $input_read (param i32 i32) (result i32)))
  (import "flame" "output" (func $output (param i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "Hello, ")
  (data (i32.const 512) "FlamePanel")
  (data (i32.const 528) "hello plugin invoked")
  (func (export "run") (result i32)
    (local $n i32)
    (call $log (i32.const 2) (i32.const 528) (i32.const 20))
    (local.set $n (call $input_len))
    (if (i32.gt_u (local.get $n) (i32.const 256)) (then (local.set $n (i32.const 256))))
    (if (i32.eqz (local.get $n))
      (then
        (memory.copy (i32.const 7) (i32.const 512) (i32.const 10))
        (local.set $n (i32.const 10)))
      (else (drop (call $input_read (i32.const 7) (local.get $n)))))
    (i32.store8 (i32.add (i32.const 7) (local.get $n)) (i32.const 33))
    (call $output (i32.const 0) (i32.add (local.get $n) (i32.const 8)))))
"#;

/// 内置插件：metrics-snapshot —— `metrics:read`；输出面板本机最新指标快照 JSON。
pub const METRICS_WAT: &str = r#"(module
  (import "flame" "metrics" (func $metrics (param i32 i32) (result i32)))
  (import "flame" "output" (func $output (param i32 i32) (result i32)))
  (memory (export "memory") 2)
  (data (i32.const 0) "{\"error\":\"metrics unavailable\"}")
  (func (export "run") (result i32)
    (local $n i32)
    (local.set $n (call $metrics (i32.const 1024) (i32.const 130048)))
    (if (i32.lt_s (local.get $n) (i32.const 0))
      (then
        (drop (call $output (i32.const 0) (i32.const 31)))
        (return (local.get $n))))
    (if (i32.gt_u (local.get $n) (i32.const 130048)) (then (local.set $n (i32.const 130048))))
    (drop (call $output (i32.const 1024) (local.get $n)))
    (i32.const 0)))
"#;

/// 内置插件：log-viewer —— `fs:read:/var/log`；输入为空列出 /var/log，否则读取输入路径
/// （相对 /var/log；目录则列出），演示 fs_read / fs_list 与能力根约束。
pub const LOG_VIEWER_WAT: &str = r#"(module
  (import "flame" "input_len" (func $input_len (result i32)))
  (import "flame" "input_read" (func $input_read (param i32 i32) (result i32)))
  (import "flame" "fs_read" (func $fs_read (param i32 i32 i32 i32) (result i32)))
  (import "flame" "fs_list" (func $fs_list (param i32 i32 i32 i32) (result i32)))
  (import "flame" "output" (func $output (param i32 i32) (result i32)))
  (memory (export "memory") 18)
  (data (i32.const 4096) "/var/log")
  (data (i32.const 4112) "error: access denied, not found or unreadable")
  (func (export "run") (result i32)
    (local $n i32) (local $r i32)
    (local.set $n (call $input_len))
    (if (i32.gt_u (local.get $n) (i32.const 4096)) (then (return (i32.const -2))))
    (if (i32.eqz (local.get $n))
      (then
        (local.set $r (call $fs_list (i32.const 4096) (i32.const 8) (i32.const 65536) (i32.const 1048576))))
      (else
        (drop (call $input_read (i32.const 0) (local.get $n)))
        (local.set $r (call $fs_read (i32.const 0) (local.get $n) (i32.const 65536) (i32.const 1048576)))
        (if (i32.eq (local.get $r) (i32.const -2))
          (then
            (local.set $r (call $fs_list (i32.const 0) (local.get $n) (i32.const 65536) (i32.const 1048576)))))))
    (if (i32.lt_s (local.get $r) (i32.const 0))
      (then
        (drop (call $output (i32.const 4112) (i32.const 45)))
        (return (local.get $r))))
    (if (i32.gt_u (local.get $r) (i32.const 1048576)) (then (local.set $r (i32.const 1048576))))
    (drop (call $output (i32.const 65536) (local.get $r)))
    (i32.const 0)))
"#;

struct Builtin {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    capabilities: &'static [&'static str],
    tags: &'static [&'static str],
    wat: &'static str,
}

const BUILTINS: &[Builtin] = &[
    Builtin {
        id: "flame-hello",
        name: "Hello",
        description: "Greets the given input. Demonstrates log / input / output host calls.",
        capabilities: &[],
        tags: &["example"],
        wat: HELLO_WAT,
    },
    Builtin {
        id: "flame-metrics-snapshot",
        name: "Metrics Snapshot",
        description: "Returns the latest local metrics snapshot as JSON.",
        capabilities: &["metrics:read"],
        tags: &["example", "monitoring"],
        wat: METRICS_WAT,
    },
    Builtin {
        id: "flame-log-viewer",
        name: "Log Viewer",
        description: "Lists /var/log or reads a log file (path relative to /var/log, max 1 MiB).",
        capabilities: &["fs:read:/var/log"],
        tags: &["example", "logs"],
        wat: LOG_VIEWER_WAT,
    },
];

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// 编译内置插件 WAT 为 wasm 字节码（需要 feature `wasm`）。
#[cfg(feature = "wasm")]
fn compile_wat(wat_src: &str) -> Result<Vec<u8>, AppError> {
    wat::parse_str(wat_src).map_err(|e| AppError::internal(format!("builtin plugin WAT: {e}")))
}

#[cfg(not(feature = "wasm"))]
fn compile_wat(_wat_src: &str) -> Result<Vec<u8>, AppError> {
    Err(AppError::ServiceUnavailable(
        "WASM support disabled (feature `wasm` is off)".to_string(),
    ))
}

/// 内置目录。`wasm_sha256` 为编译产物哈希（未开启 wasm feature 时为空）。
pub fn builtin_catalog() -> Vec<MarketPlugin> {
    BUILTINS
        .iter()
        .map(|b| MarketPlugin {
            id: b.id.into(),
            name: b.name.into(),
            version: "1.0.0".into(),
            author: "FlamePanel".into(),
            description: b.description.into(),
            capabilities: b.capabilities.iter().map(|c| c.to_string()).collect(),
            tags: b.tags.iter().map(|t| t.to_string()).collect(),
            homepage: None,
            license: Some("MIT".into()),
            entry: default_entry(),
            wasm_url: None,
            wasm_sha256: compile_wat(b.wat)
                .map(|w| sha256_hex(&w))
                .unwrap_or_default(),
            source: MarketSource::Builtin,
        })
        .collect()
}

/// 内置插件字节码。
pub fn builtin_wasm(id: &str) -> Result<Vec<u8>, AppError> {
    let b = BUILTINS
        .iter()
        .find(|b| b.id == id)
        .ok_or_else(|| AppError::NotFound(format!("market plugin {id} not found")))?;
    compile_wat(b.wat)
}

fn validate_url(url: &str) -> Result<(), AppError> {
    if !(url.starts_with("https://") || url.starts_with("http://")) || url.len() > 2048 {
        return Err(AppError::ValidationError(format!(
            "plugin market url must be http(s): {url}"
        )));
    }
    Ok(())
}

fn http_client() -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .map_err(|e| AppError::internal(format!("http client: {e}")))
}

/// 流式下载并在超过上限时中止（不信任 Content-Length）。
async fn fetch_limited(url: &str, limit: usize) -> Result<Vec<u8>, AppError> {
    validate_url(url)?;
    let mut resp = http_client()?
        .get(url)
        .send()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("fetch {url}: {e}")))?;
    if !resp.status().is_success() {
        return Err(AppError::ServiceUnavailable(format!(
            "fetch {url}: HTTP {}",
            resp.status()
        )));
    }
    let mut buf = Vec::new();
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("fetch {url}: {e}")))?
    {
        if buf.len() + chunk.len() > limit {
            return Err(AppError::BadRequest(format!(
                "{url}: response exceeds {limit} bytes"
            )));
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(buf)
}

/// 解析并校验远程索引（纯函数，便于单测）。非法条目跳过，不影响其余条目。
pub fn parse_remote_index(body: &[u8]) -> Result<Vec<MarketPlugin>, AppError> {
    let idx: RemoteIndex = serde_json::from_slice(body)
        .map_err(|e| AppError::BadRequest(format!("invalid plugin market index: {e}")))?;
    let builtin_ids: Vec<&str> = BUILTINS.iter().map(|b| b.id).collect();
    Ok(idx
        .plugins
        .into_iter()
        .filter(|p| {
            let sha_ok =
                p.wasm_sha256.len() == 64 && p.wasm_sha256.bytes().all(|b| b.is_ascii_hexdigit());
            let url_ok = p
                .wasm_url
                .as_deref()
                .is_some_and(|u| validate_url(u).is_ok());
            let id_ok = !p.id.is_empty()
                && p.id.len() <= 64
                && p.id != "market"
                && p.id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                && !builtin_ids.contains(&p.id.as_str());
            let caps_ok = super::capability::Capabilities::parse_all(&p.capabilities).is_ok();
            sha_ok && url_ok && id_ok && caps_ok
        })
        .take(500)
        .map(|mut p| {
            p.source = MarketSource::Remote;
            p.wasm_sha256 = p.wasm_sha256.to_ascii_lowercase();
            p
        })
        .collect())
}

pub async fn fetch_remote_index(url: &str) -> Result<Vec<MarketPlugin>, AppError> {
    let body = fetch_limited(url, MAX_INDEX_BYTES).await?;
    parse_remote_index(&body)
}

/// 下载远程包体并强制校验 SHA-256。
pub async fn fetch_remote_wasm(entry: &MarketPlugin) -> Result<Vec<u8>, AppError> {
    let url = entry
        .wasm_url
        .as_deref()
        .ok_or_else(|| AppError::BadRequest("market entry has no wasm_url".into()))?;
    let bytes = fetch_limited(url, MAX_WASM_BYTES).await?;
    let actual = sha256_hex(&bytes);
    if actual != entry.wasm_sha256 {
        return Err(AppError::BadRequest(format!(
            "WASM hash mismatch: expected {}, got {actual}",
            entry.wasm_sha256
        )));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_catalog_capabilities_are_valid() {
        let cat = builtin_catalog();
        assert_eq!(cat.len(), 3);
        for p in &cat {
            assert!(super::super::capability::Capabilities::parse_all(&p.capabilities).is_ok());
            assert_eq!(p.source, MarketSource::Builtin);
        }
        assert!(builtin_wasm("nope").is_err());
    }

    #[cfg(feature = "wasm")]
    #[test]
    fn builtin_wat_compiles_and_hash_matches() {
        for p in builtin_catalog() {
            let wasm = builtin_wasm(&p.id).unwrap();
            assert_eq!(&wasm[..4], b"\0asm");
            assert_eq!(sha256_hex(&wasm), p.wasm_sha256);
        }
    }

    #[test]
    fn remote_index_filters_invalid_entries() {
        let sha = "a".repeat(64);
        let body = serde_json::json!({"plugins": [
            {"id": "ok-1", "name": "Ok", "version": "1.0.0", "wasm_url": "https://x/p.wasm", "wasm_sha256": sha.to_uppercase(), "capabilities": ["metrics:read"]},
            {"id": "bad-sha", "name": "B", "version": "1", "wasm_url": "https://x/p.wasm", "wasm_sha256": "abc"},
            {"id": "bad-url", "name": "B", "version": "1", "wasm_url": "file:///etc/passwd", "wasm_sha256": sha},
            {"id": "bad-cap", "name": "B", "version": "1", "wasm_url": "https://x/p.wasm", "wasm_sha256": sha, "capabilities": ["fs:read:/"]},
            {"id": "flame-hello", "name": "Shadow", "version": "9", "wasm_url": "https://x/p.wasm", "wasm_sha256": sha},
            {"id": "../evil", "name": "B", "version": "1", "wasm_url": "https://x/p.wasm", "wasm_sha256": sha}
        ]});
        let list = parse_remote_index(body.to_string().as_bytes()).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "ok-1");
        assert_eq!(list[0].source, MarketSource::Remote);
        assert_eq!(list[0].wasm_sha256, sha);
        assert_eq!(list[0].entry, "run");
        assert!(parse_remote_index(b"not json").is_err());
    }

    #[test]
    fn url_validation() {
        assert!(validate_url("https://example.com/index.json").is_ok());
        assert!(validate_url("file:///etc/passwd").is_err());
        assert!(validate_url("ftp://x").is_err());
    }
}
