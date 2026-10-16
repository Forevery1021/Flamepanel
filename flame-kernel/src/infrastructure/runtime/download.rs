//! 运行时发行包下载与架构探测（M-C Stage B-1/B-2 · Doc/20 §5.3）。
//!
//! Go/Node 等「官方发行包目录式安装」语言共用同一份下载与架构归一逻辑：
//! - [`fetch`]：带超时的字节流下载，非 2xx 明确报错（不把错误页当包体）；
//! - [`uname_arch`]：`uname -m` 探测 + 官方命名归一（x86_64→amd64 / aarch64→arm64）；
//! - [`release_base`]：发行包脚手架目录（幂等创建，解包目标）。
//!
//! 全部经 [`SharedCommandRunner`] 执行（`execution_mode=embedded|agent` 自适应），
//! 与 `webserver/native.rs` 的发行版感知路由风格一致。

use crate::application::execution_mode::{PrivilegedCommand, SharedCommandRunner};
use crate::core::error::AppError;
use std::path::Path;

/// 官方发行包下载超时（秒；Node/Go 发行包数十 MB，弱网留足余量）。
pub const DOWNLOAD_TIMEOUT_SECS: u64 = 600;

/// 已落盘的下载产物：Drop 时自动删除临时文件（成功/失败路径都不留残包）。
pub struct DownloadedFile {
    path: std::path::PathBuf,
}

impl DownloadedFile {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for DownloadedFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// 临时文件唯一序号（同进程并发安装互不覆盖）。
static DL_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// 流式下载 URL 到临时文件（非 2xx 明确报错，附带 URL 便于排查镜像/版本问题）。
///
/// 发行包数十 MB 且解包入口（[`super::archive`]）支持文件读取，边下边写盘
/// 避免整包驻留内存（512 MB 目标环境下内存峰值是硬约束）；HTTP 客户端复用
/// [`crate::infrastructure::http::shared_client`]，不再每次下载重建连接池。
pub async fn fetch(url: &str, what: &str) -> Result<DownloadedFile, AppError> {
    use tokio::io::AsyncWriteExt;
    let mut resp = crate::infrastructure::http::shared_client()
        .get(url)
        .timeout(std::time::Duration::from_secs(DOWNLOAD_TIMEOUT_SECS))
        .send()
        .await
        .map_err(|e| AppError::internal(format!("下载{}失败: {}", what, e)))?;
    if !resp.status().is_success() {
        return Err(AppError::internal(format!(
            "下载{}失败: HTTP {}（{}）",
            what,
            resp.status(),
            url
        )));
    }
    let path = std::env::temp_dir().join(format!(
        "fp-runtime-dl-{}-{}.part",
        std::process::id(),
        DL_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    // 先建 guard：后续任一步骤失败，Drop 兜底清理半截文件
    let guard = DownloadedFile { path };
    let mut file = tokio::fs::File::create(guard.path())
        .await
        .map_err(|e| AppError::internal(format!("创建{}临时文件失败: {}", what, e)))?;
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| AppError::internal(format!("读取{}失败: {}", what, e)))?
    {
        file.write_all(&chunk)
            .await
            .map_err(|e| AppError::internal(format!("写入{}临时文件失败: {}", what, e)))?;
    }
    file.flush()
        .await
        .map_err(|e| AppError::internal(format!("写入{}临时文件失败: {}", what, e)))?;
    Ok(guard)
}

/// `uname -m` → 官方发行包架构命名（Go/Node 共用 amd64/arm64 归一）。
pub async fn uname_arch(
    runner: &SharedCommandRunner,
    what: &str,
) -> Result<&'static str, AppError> {
    let out = runner
        .run(&PrivilegedCommand::new("uname", vec!["-m".into()]))
        .await?;
    normalize_arch(&out.stdout).ok_or_else(|| {
        AppError::BadRequest(format!(
            "不支持的 CPU 架构: {}（{}暂无对应发行包）",
            out.stdout.trim(),
            what
        ))
    })
}

/// `uname -m` 输出 → 官方命名（纯函数，便于单测）。
pub fn normalize_arch(uname: &str) -> Option<&'static str> {
    match uname.trim() {
        "x86_64" | "amd64" => Some("amd64"),
        "aarch64" | "arm64" => Some("arm64"),
        _ => None,
    }
}

/// 内容摘要（SHA-256 十六进制，`sha2` 既有依赖）：用于 Python 依赖
/// 安装的「requirements.txt 未变化则跳过 pip」幂等判定。
pub fn content_digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

/// 发行包安装脚手架目录（幂等创建）。
pub fn ensure_dir(path: &Path) -> Result<(), AppError> {
    if path.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(path)
        .map_err(|e| AppError::internal(format!("创建运行时目录失败: {}", e)))
}

// ─── 目录式安装共用工具（Go / Java / Python 与 Node 同构，M-C 收尾） ─────────────

/// 镜像基址：环境变量值为 https 地址时采用（去尾 `/`），否则回落官方源并告警。
pub fn mirror_from(value: Option<&str>, default: &str, env_name: &str) -> String {
    match value.map(str::trim).filter(|v| !v.is_empty()) {
        Some(v) if v.starts_with("https://") => v.trim_end_matches('/').to_string(),
        Some(v) => {
            tracing::warn!("{env_name}={v:?} 不是 https 地址，已忽略并使用官方源");
            default.to_string()
        }
        None => default.to_string(),
    }
}

/// 读取环境变量形式的镜像配置（见 [`mirror_from`]）。
pub fn mirror_env(env_name: &str, default: &str) -> String {
    mirror_from(std::env::var(env_name).ok().as_deref(), default, env_name)
}

/// `1` / `1.23` / `1.23.4` → 数字段（1–3 段；非数字即 None）。
pub fn version_parts(v: &str) -> Option<Vec<u64>> {
    let parts: Option<Vec<u64>> = v.trim().split('.').map(|p| p.parse().ok()).collect();
    parts.filter(|p| (1..=3).contains(&p.len()))
}

/// 流式计算文件 SHA-256（64 KiB 分块，不整包驻留内存）。
pub fn file_sha256(path: &Path) -> Result<String, AppError> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = std::fs::File::open(path)
        .map_err(|e| AppError::internal(format!("读取安装包失败: {e}")))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| AppError::internal(format!("读取安装包失败: {e}")))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// 校验下载产物的 SHA-256（阻塞计算放到 blocking 线程池）；不符即拒装。
pub async fn verify_sha256(
    file: &DownloadedFile,
    expected: &str,
    what: &str,
) -> Result<(), AppError> {
    let expected = expected.trim().to_ascii_lowercase();
    if expected.len() != 64 || !expected.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::internal(format!(
            "{what}的校验和格式异常: {expected:?}"
        )));
    }
    let path = file.path().to_path_buf();
    let actual = tokio::task::spawn_blocking(move || file_sha256(&path))
        .await
        .map_err(|e| AppError::internal(format!("校验{what}失败: {e}")))??;
    if actual != expected {
        return Err(AppError::internal(format!(
            "{what} SHA-256 校验失败（期望 {expected}，实际 {actual}），已拒绝安装"
        )));
    }
    Ok(())
}

/// `<hex>  <文件名>` 或只有 `<hex>` 的校验和文本 → 小写十六进制（取首个 64 位十六进制词）。
pub fn parse_sha256_text(text: &str) -> Option<String> {
    text.split_whitespace()
        .next()
        .filter(|h| h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit()))
        .map(str::to_ascii_lowercase)
}

/// 把 `staging/` 下**唯一的顶层目录**（发行包的包裹目录，如 `go/`、`jdk-21.0.4+7/`）的内容
/// 上提到 `root`，随后删除 `staging`。同文件系统 rename，原子且零拷贝。
///
/// 顶层不是恰好一个目录时（发行包结构异常）报错，不做猜测。
pub fn flatten_single_dir(root: &Path, staging: &Path, what: &str) -> Result<(), AppError> {
    let tops: Vec<std::path::PathBuf> = std::fs::read_dir(staging)
        .map_err(|e| AppError::internal(format!("{what}解包目录不可读: {e}")))?
        .flatten()
        .map(|e| e.path())
        .collect();
    let [top] = tops.as_slice() else {
        return Err(AppError::internal(format!(
            "{what}发行包结构异常：顶层应为单个目录，实际 {} 项",
            tops.len()
        )));
    };
    if !top.is_dir() {
        return Err(AppError::internal(format!(
            "{what}发行包结构异常：顶层不是目录"
        )));
    }
    for entry in std::fs::read_dir(top)
        .map_err(|e| AppError::internal(format!("{what}解包目录不可读: {e}")))?
        .flatten()
    {
        let target = root.join(entry.file_name());
        if target.is_dir() {
            let _ = std::fs::remove_dir_all(&target);
        } else if target.exists() {
            let _ = std::fs::remove_file(&target);
        }
        std::fs::rename(entry.path(), &target)
            .map_err(|e| AppError::internal(format!("整理{what}安装目录失败: {e}")))?;
    }
    let _ = std::fs::remove_dir_all(staging);
    Ok(())
}

/// 以若干运行时 `bin/` 打头的 PATH（守护单元不经登录 shell，需显式给出）。
pub fn path_with_bins(bins: &[&str]) -> String {
    let mut parts: Vec<&str> = bins.iter().copied().filter(|b| !b.is_empty()).collect();
    parts.extend([
        "/usr/local/sbin",
        "/usr/local/bin",
        "/usr/sbin",
        "/usr/bin",
        "/sbin",
        "/bin",
    ]);
    parts.join(":")
}

/// 读取安装记录 `install_path/<file>`（不存在 / 损坏为 None）。
pub fn read_marker(install_path: &str, file: &str) -> Option<serde_json::Value> {
    let raw = std::fs::read_to_string(Path::new(install_path).join(file)).ok()?;
    serde_json::from_str(&raw).ok()
}

/// 写安装记录（尽力而为：记录只用于展示与幂等判断，写失败不影响安装结果）。
pub fn write_marker(install_path: &str, file: &str, value: &serde_json::Value) {
    let _ = std::fs::write(Path::new(install_path).join(file), value.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_arch_maps_uname_variants() {
        assert_eq!(normalize_arch("x86_64\n"), Some("amd64"));
        assert_eq!(normalize_arch("amd64"), Some("amd64"));
        assert_eq!(normalize_arch("aarch64"), Some("arm64"));
        assert_eq!(normalize_arch("arm64"), Some("arm64"));
        assert_eq!(normalize_arch("mips"), None);
        assert_eq!(normalize_arch(""), None);
    }

    #[test]
    fn content_digest_is_stable_and_sensitive_to_content() {
        let a = content_digest(b"fastapi\nuvicorn\n");
        assert_eq!(a.len(), 64, "SHA-256 十六进制长度");
        assert_eq!(a, content_digest(b"fastapi\nuvicorn\n"), "同内容同摘要");
        assert_ne!(a, content_digest(b"fastapi\nuvicorn"), "内容变化摘要变化");
    }

    #[test]
    fn ensure_dir_is_idempotent() {
        let dir = std::env::temp_dir().join(format!("fp-dl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        ensure_dir(&dir).unwrap();
        ensure_dir(&dir).unwrap();
        assert!(dir.is_dir());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn tmp(tag: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "fp-dl-{tag}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn mirror_accepts_https_only() {
        let d = "https://go.dev/dl";
        assert_eq!(mirror_from(None, d, "X"), d);
        assert_eq!(mirror_from(Some("  "), d, "X"), d);
        assert_eq!(
            mirror_from(Some("https://mirrors.aliyun.com/golang/"), d, "X"),
            "https://mirrors.aliyun.com/golang"
        );
        assert_eq!(mirror_from(Some("http://evil.invalid"), d, "X"), d);
        assert_eq!(mirror_from(Some("file:///etc"), d, "X"), d);
    }

    #[test]
    fn version_parts_accepts_one_to_three_numeric_segments() {
        assert_eq!(version_parts("1"), Some(vec![1]));
        assert_eq!(version_parts("1.23"), Some(vec![1, 23]));
        assert_eq!(version_parts("1.23.4"), Some(vec![1, 23, 4]));
        assert_eq!(version_parts("1.2.3.4"), None);
        assert_eq!(version_parts("1.x"), None);
        assert_eq!(version_parts("../1"), None);
        assert_eq!(version_parts(""), None);
    }

    #[test]
    fn parse_sha256_text_takes_first_hex_word() {
        let h = "a".repeat(64);
        assert_eq!(
            parse_sha256_text(&format!("{h}  go1.23.4.linux-amd64.tar.gz\n")),
            Some(h.clone())
        );
        assert_eq!(parse_sha256_text(&h.to_uppercase()), Some(h));
        assert_eq!(parse_sha256_text("<html>404</html>"), None);
        assert_eq!(parse_sha256_text(""), None);
    }

    #[tokio::test]
    async fn verify_sha256_accepts_match_and_rejects_mismatch() {
        let dir = tmp("sha");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("pkg.part");
        std::fs::write(&path, b"flamepanel").unwrap();
        let f = DownloadedFile { path };
        let good = content_digest(b"flamepanel");
        verify_sha256(&f, &good, "测试包").await.unwrap();
        verify_sha256(&f, &good.to_uppercase(), "测试包")
            .await
            .unwrap();
        let err = verify_sha256(&f, &"0".repeat(64), "测试包")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("校验失败"), "{err}");
        assert!(
            verify_sha256(&f, "abc", "测试包").await.is_err(),
            "格式异常拒绝"
        );
        drop(f);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn flatten_single_dir_lifts_wrapper_contents() {
        let root = tmp("flat");
        let staging = root.join(".staging");
        std::fs::create_dir_all(staging.join("jdk-21.0.4+7/bin")).unwrap();
        std::fs::write(staging.join("jdk-21.0.4+7/bin/java"), "").unwrap();
        std::fs::write(staging.join("jdk-21.0.4+7/release"), "JAVA_VERSION=21").unwrap();
        // 旧文件被替换
        std::fs::create_dir_all(root.join("bin")).unwrap();
        std::fs::write(root.join("bin/stale"), "").unwrap();
        flatten_single_dir(&root, &staging, "JDK").unwrap();
        assert!(root.join("bin/java").is_file());
        assert!(root.join("release").is_file());
        assert!(!root.join("bin/stale").exists());
        assert!(!staging.exists(), "staging 已清理");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn flatten_single_dir_rejects_unexpected_layout() {
        let root = tmp("flat-bad");
        let staging = root.join(".staging");
        std::fs::create_dir_all(staging.join("a")).unwrap();
        std::fs::create_dir_all(staging.join("b")).unwrap();
        assert!(flatten_single_dir(&root, &staging, "X").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn path_with_bins_prepends_in_order() {
        assert_eq!(
            path_with_bins(&["/a/bin", "", "/b/bin"]),
            "/a/bin:/b/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
        );
    }

    #[test]
    fn marker_roundtrip_and_missing() {
        let dir = tmp("marker");
        std::fs::create_dir_all(&dir).unwrap();
        let d = dir.to_string_lossy().to_string();
        assert!(read_marker(&d, ".fp-x.json").is_none());
        write_marker(&d, ".fp-x.json", &serde_json::json!({"version": "1.23.4"}));
        assert_eq!(read_marker(&d, ".fp-x.json").unwrap()["version"], "1.23.4");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
