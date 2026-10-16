//! 远程应用源同步器（M-A Issue 1.2，Doc/20 §4.1）
//!
//! `RemoteAppSource` 端口的两个实现：
//! - [`GitRemoteSource`]：git clone --depth 1 拉取仓库（1Panel appstore 仓库镜像），
//!   同步 = 拉全量索引目录后用适配器解析每个应用包；
//! - [`HttpRemoteSource`]：HTTP JSON manifest（自建源协议），一次 GET 拉取索引。
//!
//! 懒加载原则：同步阶段只建索引（AppPackage 落库），安装时才按需下载包体。

use crate::application::app_store_ports::AppAdapterProvider;
use crate::application::execution_mode::SharedCommandRunner;
use crate::core::error::AppError;
use crate::domain::entity::{AppPackage, AppSource};
use async_trait::async_trait;
use chrono::Utc;
use std::path::{Path, PathBuf};

/// 一次远程源同步的结果。
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct SyncReport {
    /// 本次新增包数
    pub added: usize,
    /// 已存在跳过数（幂等）
    pub skipped: usize,
    /// 解析成功并纳入索引的包 key
    pub packages: Vec<String>,
}

/// 远程源同步端口（Doc/20 §4.1：sync_index 索引同步 / fetch_package 包体按需拉取）。
#[async_trait]
pub trait RemoteAppSource: Send + Sync {
    /// 同步索引：把远程源的应用包索引落到本地，返回可落库的 AppPackage 列表。
    async fn sync_index(&self, source: &AppSource) -> Result<Vec<AppPackage>, AppError>;

    /// 按需拉取单个应用包体到本地缓存目录（返回包根路径）。
    async fn fetch_package(
        &self,
        source: &AppSource,
        key: &str,
        cache_root: &Path,
    ) -> Result<PathBuf, AppError>;
}

/// 装箱的远程源实例（application 层签名时避免 `dyn` trait 对象双层装箱问题）。
pub type BoxedRemoteSource = Box<dyn RemoteAppSource>;

/// url 校验：仅允许 https（可选 http 用于内网自建源），禁止本地文件等协议。
fn validate_url(url: &str) -> Result<(), AppError> {
    let ok = url.starts_with("https://") || url.starts_with("http://");
    if !ok {
        return Err(AppError::ValidationError(format!(
            "远程源 url 必须以 https:// 或 http:// 开头: {}",
            url
        )));
    }
    Ok(())
}

/// 应用包 key 校验：key 会拼进商店目录（`apps_dir.join(key)`），必须是单个安全路径段。
/// 字符集 `[A-Za-z0-9._-]`、1–128 位、不以 `.` 开头（排除 `.` / `..` 与隐藏目录）。
/// 远程源与本地导入共用。
pub(crate) fn validate_key(key: &str) -> Result<(), AppError> {
    let valid = !key.is_empty()
        && key.len() <= 128
        && !key.starts_with('.')
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if !valid {
        return Err(AppError::ValidationError(format!(
            "非法应用包 key: {}",
            key
        )));
    }
    Ok(())
}

// ─── Git 远程源 ─────────────────────────────────────────────────────────────

/// Git 仓库远程源：`git clone --depth 1` 懒加载。
///
/// 同步语义：clone 到临时目录 → 遍历一级子目录（每个子目录即一个应用包）→
/// 经 `AppAdapterProvider` 识别格式并解析元数据 → 生成索引 AppPackage 列表
/// （source_path 指向 clone 缓存目录，安装时直接复用，无需二次下载）。
pub struct GitRemoteSource {
    runner: SharedCommandRunner,
    adapter_provider: std::sync::Arc<dyn AppAdapterProvider>,
    /// 克隆缓存根目录（如 data/apps/remote/<source_id>）
    pub cache_root: PathBuf,
}

impl GitRemoteSource {
    pub fn new(
        runner: SharedCommandRunner,
        adapter_provider: std::sync::Arc<dyn AppAdapterProvider>,
        cache_root: PathBuf,
    ) -> Self {
        Self {
            runner,
            adapter_provider,
            cache_root,
        }
    }

    fn repo_dir(&self, source: &AppSource) -> PathBuf {
        self.cache_root.join(format!("source-{}", source.id))
    }

    async fn git(&self, args: &[&str], timeout: u64) -> Result<String, AppError> {
        let cmd = crate::domain::execution_mode::PrivilegedCommand::new(
            "git",
            args.iter().map(|s| s.to_string()).collect(),
        )
        .timeout(timeout);
        let output = self.runner.run(&cmd).await?;
        if !output.success() {
            return Err(AppError::internal(format!(
                "git 命令失败: {}",
                output.combined()
            )));
        }
        Ok(output.stdout)
    }
}

#[async_trait]
impl RemoteAppSource for GitRemoteSource {
    async fn sync_index(&self, source: &AppSource) -> Result<Vec<AppPackage>, AppError> {
        validate_url(&source.url)?;
        let repo_dir = self.repo_dir(source);

        // 已有 clone 则增量拉取，否则浅克隆（懒加载：不带历史）
        if repo_dir.join(".git").is_dir() {
            self.git(
                &["-C", &repo_dir.to_string_lossy(), "pull", "--ff-only"],
                300,
            )
            .await?;
        } else {
            std::fs::create_dir_all(&repo_dir)
                .map_err(|e| AppError::internal(format!("创建缓存目录失败: {}", e)))?;
            self.git(
                &[
                    "clone",
                    "--depth",
                    "1",
                    &source.url,
                    &repo_dir.to_string_lossy(),
                ],
                600,
            )
            .await?;
        }

        // 一级子目录 = 应用包（1Panel appstore 仓库布局）
        let entries = std::fs::read_dir(&repo_dir)
            .map_err(|e| AppError::internal(format!("读取仓库目录失败: {}", e)))?;
        let mut packages = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() || path.join(".git").exists() {
                continue;
            }
            let Ok(adapter) = self.adapter_provider.select(&path) else {
                continue; // 非应用包目录（如 docs/.github）静默跳过
            };
            let Ok(metadata) = adapter.parse_metadata(&path) else {
                continue; // 解析失败的包跳过，不阻断整体同步
            };
            packages.push(AppPackage {
                id: 0,
                key: metadata.key.clone(),
                name: metadata.name.clone(),
                category: metadata.category.clone(),
                format: metadata.format.as_str().into(),
                description: metadata.short_desc_zh.clone(),
                logo: metadata.logo.clone(),
                metadata_json: serde_json::to_string(&metadata)
                    .map_err(|e| AppError::internal(format!("serialize metadata: {}", e)))?,
                source_path: Some(path.to_string_lossy().into_owned()),
                source: "git".to_string(),
                source_url: Some(source.url.clone()),
                synced_at: Some(Utc::now()),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            });
        }
        Ok(packages)
    }

    async fn fetch_package(
        &self,
        source: &AppSource,
        key: &str,
        _cache_root: &Path,
    ) -> Result<PathBuf, AppError> {
        validate_key(key)?;
        let repo_dir = self.repo_dir(source);
        if !repo_dir.is_dir() {
            return Err(AppError::NotFound(
                "远程源尚未同步索引，请先执行同步".into(),
            ));
        }
        let pkg_dir = repo_dir.join(key);
        if !pkg_dir.is_dir() {
            return Err(AppError::NotFound(format!("远程源中不存在应用包: {}", key)));
        }
        Ok(pkg_dir)
    }
}

// ─── HTTP 远程源 ────────────────────────────────────────────────────────────

/// HTTP JSON manifest 源（自建源协议）。
///
/// 索引格式（GET {url}）：
/// ```json
/// { "packages": [ { "key": "gitea", "name": "Gitea", "category": "devops",
///     "description": "...", "format": "flame", "download_url": "https://.../gitea.tar.gz",
///     "versions": ["1.21.0"], "logo": "https://..." } ] }
/// ```
///
/// 懒加载：同步只落索引（source_path = None）；安装时 fetch_package 下载包体
/// 解压到 cache_root/<key> 后再走适配器链路。
pub struct HttpRemoteSource {
    /// 包体下载缓存根目录
    pub cache_root: PathBuf,
}

#[derive(Debug, serde::Deserialize)]
struct HttpManifest {
    #[serde(default)]
    packages: Vec<HttpManifestEntry>,
}

#[derive(Debug, serde::Deserialize)]
struct HttpManifestEntry {
    key: String,
    name: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    description: String,
    #[serde(default = "default_format")]
    format: String,
    download_url: String,
    #[serde(default)]
    versions: Vec<String>,
    #[serde(default)]
    logo: Option<String>,
}

fn default_format() -> String {
    "flame".into()
}

impl HttpRemoteSource {
    pub fn new(cache_root: PathBuf) -> Self {
        Self { cache_root }
    }

    async fn get_json(url: &str) -> Result<HttpManifest, AppError> {
        let resp = crate::infrastructure::http::shared_client()
            .get(url)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| AppError::internal(format!("拉取远程索引失败: {}", e)))?;
        if !resp.status().is_success() {
            return Err(AppError::internal(format!(
                "拉取远程索引失败: HTTP {}",
                resp.status()
            )));
        }
        resp.json::<HttpManifest>()
            .await
            .map_err(|e| AppError::internal(format!("解析远程索引失败: {}", e)))
    }
}

#[async_trait]
impl RemoteAppSource for HttpRemoteSource {
    async fn sync_index(&self, source: &AppSource) -> Result<Vec<AppPackage>, AppError> {
        validate_url(&source.url)?;
        let manifest = Self::get_json(&source.url).await?;
        let mut packages = Vec::new();
        for entry in manifest.packages {
            validate_key(&entry.key)?;
            let category = if entry.category.is_empty() {
                "uncategorized".to_string()
            } else {
                entry.category.clone()
            };
            let default_version = entry.versions.last().cloned().unwrap_or_default();
            let metadata_json = serde_json::json!({
                "key": entry.key,
                "name": entry.name,
                "category": category,
                "short_desc_zh": entry.description,
                "short_desc_en": serde_json::Value::Null,
                "tags": [],
                "format": entry.format,
                "modes": ["container"],
                "versions": entry.versions,
                "default_version": default_version,
                "logo": entry.logo,
                "recommended": false,
            })
            .to_string();
            packages.push(AppPackage {
                id: 0,
                key: entry.key,
                name: entry.name,
                category,
                format: entry.format,
                description: entry.description,
                logo: entry.logo,
                metadata_json,
                source_path: None, // 懒加载：安装时才下载包体
                source: "http".to_string(),
                source_url: Some(source.url.clone()),
                synced_at: Some(Utc::now()),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            });
        }
        Ok(packages)
    }

    async fn fetch_package(
        &self,
        source: &AppSource,
        key: &str,
        _cache_root: &Path,
    ) -> Result<PathBuf, AppError> {
        validate_key(key)?;
        let manifest = Self::get_json(&source.url).await?;
        let entry = manifest
            .packages
            .into_iter()
            .find(|p| p.key == key)
            .ok_or_else(|| AppError::NotFound(format!("远程源中不存在应用包: {}", key)))?;
        validate_url(&entry.download_url)?;

        let dest = self.cache_root.join(&entry.key);
        if dest.is_dir() {
            // 已缓存：同样下钻单层包裹（与首次解包语义一致，幂等）
            return Ok(drill_down_single_dir(&dest));
        }
        std::fs::create_dir_all(&dest)
            .map_err(|e| AppError::internal(format!("创建缓存目录失败: {}", e)))?;

        let resp = crate::infrastructure::http::shared_client()
            .get(&entry.download_url)
            .timeout(std::time::Duration::from_secs(600))
            .send()
            .await
            .map_err(|e| AppError::internal(format!("下载应用包失败: {}", e)))?;
        if !resp.status().is_success() {
            return Err(AppError::internal(format!(
                "下载应用包失败: HTTP {}",
                resp.status()
            )));
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AppError::internal(format!("读取应用包内容失败: {}", e)))?;
        let tar_path = self.cache_root.join(format!("{}.download", entry.key));
        std::fs::write(&tar_path, &bytes)
            .map_err(|e| AppError::internal(format!("写入应用包缓存失败: {}", e)))?;

        // 解包（tar.gz / tar）
        untar(&tar_path, &dest)?;
        let _ = std::fs::remove_file(&tar_path);

        // 解包后若只有单一路径前缀目录，则下钻一层
        let final_dir = drill_down_single_dir(&dest);
        Ok(final_dir)
    }
}

/// 解压 tar / tar.gz 到 dest。
fn untar(archive: &Path, dest: &Path) -> Result<(), AppError> {
    let file = std::fs::File::open(archive)
        .map_err(|e| AppError::internal(format!("打开包缓存失败: {}", e)))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    // 逐条目安全解包：含 `..` 路径穿越的条目会被跳过（tar crate 行为），
    // 但我们对包含穿越条目的包整体判失败，避免恶意包静默缺文件。
    for entry in archive
        .entries()
        .map_err(|e| AppError::internal(format!("读取包条目失败: {}", e)))?
    {
        let mut entry = entry.map_err(|e| AppError::internal(format!("读取包条目失败: {}", e)))?;
        let path = entry
            .path()
            .map_err(|e| AppError::internal(format!("读取包条目路径失败: {}", e)))?
            .into_owned();
        if path
            .components()
            .any(|c| c == std::path::Component::ParentDir)
        {
            return Err(AppError::ValidationError(format!(
                "应用包包含非法路径穿越条目，已拒绝解包: {}",
                path.display()
            )));
        }
        // Doc/33 A-9：应用包只含 YAML/compose/静态资源，无需任何链接。
        // 放行符号链接会让 `docker-compose.yml -> /etc/shadow` 之类的条目
        // 被内核以 root 读取并经版本详情接口回显（任意文件读取）。
        let kind = entry.header().entry_type();
        if kind.is_symlink() || kind.is_hard_link() {
            return Err(AppError::ValidationError(format!(
                "应用包不允许包含链接条目，已拒绝解包: {}",
                path.display()
            )));
        }
        let dst = dest.to_path_buf();
        entry
            .unpack_in(&dst)
            .map_err(|e| AppError::internal(format!("解压条目 {} 失败: {}", path.display(), e)))?;
    }
    Ok(())
}

/// 若解压结果只含一个目录则返回该目录（去除单层包裹），否则返回原目录。
fn drill_down_single_dir(dir: &Path) -> PathBuf {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return dir.to_path_buf();
    };
    let entries: Vec<_> = entries.flatten().collect();
    if entries.len() == 1 && entries[0].path().is_dir() {
        return entries[0].path();
    }
    dir.to_path_buf()
}

/// 按源类型构造对应同步器实例。
pub fn create_remote_source(
    source: &AppSource,
    runner: SharedCommandRunner,
    adapter_provider: std::sync::Arc<dyn AppAdapterProvider>,
    cache_root: &Path,
) -> Result<Box<dyn RemoteAppSource>, AppError> {
    match source.source_type.as_str() {
        "git" => Ok(Box::new(GitRemoteSource::new(
            runner,
            adapter_provider,
            cache_root.to_path_buf(),
        ))),
        "http" => Ok(Box::new(HttpRemoteSource::new(cache_root.to_path_buf()))),
        other => Err(AppError::ValidationError(format!(
            "不支持的远程源类型: {}（仅支持 git | http）",
            other
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git_source(url: &str) -> AppSource {
        AppSource {
            id: 1,
            source_type: "git".into(),
            url: url.into(),
            ..Default::default()
        }
    }

    #[test]
    fn rejects_non_http_url() {
        assert!(validate_url("file:///etc/passwd").is_err());
        assert!(validate_url("ftp://x").is_err());
        assert!(validate_url("/local/path").is_err());
        assert!(validate_url("https://github.com/1Panel-dev/appstore").is_ok());
        assert!(validate_url("http://192.168.1.10/manifest.json").is_ok());
    }

    #[test]
    fn rejects_injection_key() {
        assert!(validate_key("wordpress").is_ok());
        assert!(validate_key("app.v2_x").is_ok());
        assert!(validate_key("").is_err());
        assert!(validate_key("../etc").is_err());
        assert!(validate_key("..").is_err());
        assert!(validate_key(".").is_err());
        assert!(validate_key(".hidden").is_err());
        assert!(validate_key("a b;rm").is_err());
    }

    #[tokio::test]
    async fn create_remote_source_validates_type() {
        let runner = std::sync::Arc::new(crate::infrastructure::execution::EmbeddedCommandRunner);
        let provider =
            std::sync::Arc::new(crate::infrastructure::app_store::adapter::DefaultAdapterProvider);
        let mut s = git_source("https://example.com/repo.git");
        assert!(
            create_remote_source(&s, runner.clone(), provider.clone(), Path::new("/tmp")).is_ok()
        );
        s.source_type = "ftp".into();
        assert!(create_remote_source(&s, runner, provider, Path::new("/tmp")).is_err());
    }

    #[tokio::test]
    async fn git_sync_rejects_bad_url_without_network() {
        let runner = std::sync::Arc::new(crate::infrastructure::execution::EmbeddedCommandRunner);
        let provider =
            std::sync::Arc::new(crate::infrastructure::app_store::adapter::DefaultAdapterProvider);
        let src = crate::infrastructure::app_store::remote_source::GitRemoteSource::new(
            runner,
            provider,
            std::env::temp_dir().join("flame-git-test"),
        );
        let mut s = git_source("file:///etc");
        assert!(src.sync_index(&s).await.is_err());
        s.url = "https://invalid.invalid/repo.git".into();
        assert!(src.sync_index(&s).await.is_err());
    }

    #[tokio::test]
    async fn http_sync_parses_manifest() {
        // 用本地 HTTP 服务测试完整 manifest 解析
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let body = r#"{"packages":[{"key":"gitea","name":"Gitea","category":"devops","description":"Git service","format":"flame","download_url":"http://127.0.0.1:1/gitea.tar.gz","versions":["1.21.0"]}]}"#;
        tokio::spawn(async move {
            loop {
                let Ok((sock, _)) = listener.accept().await else {
                    break;
                };
                let body = body.to_string();
                tokio::spawn(async move {
                    use tokio::io::AsyncWriteExt;
                    let mut sock = sock;
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = sock.write_all(resp.as_bytes()).await;
                });
            }
        });
        let src = HttpRemoteSource::new(std::env::temp_dir().join("flame-http-test"));
        let mut s = git_source("placeholder");
        s.source_type = "http".into();
        s.url = format!("http://{}/manifest.json", addr);
        let packages = src.sync_index(&s).await.unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].key, "gitea");
        assert_eq!(packages[0].source, "http");
        let meta: serde_json::Value = serde_json::from_str(&packages[0].metadata_json).unwrap();
        assert_eq!(meta["key"], "gitea");
        assert_eq!(meta["default_version"], "1.21.0");
    }

    #[test]
    fn untar_rejects_link_entries() {
        let tmp = std::env::temp_dir().join(format!("flame-untar-link-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        for (i, kind) in [tar::EntryType::Symlink, tar::EntryType::Link]
            .into_iter()
            .enumerate()
        {
            let tar_path = tmp.join(format!("evil{i}.tar.gz"));
            {
                let f = std::fs::File::create(&tar_path).unwrap();
                let enc = flate2::write::GzEncoder::new(f, flate2::Compression::default());
                let mut b = tar::Builder::new(enc);
                let mut h = tar::Header::new_gnu();
                h.set_size(0);
                h.set_entry_type(kind);
                b.append_link(&mut h, "app/docker-compose.yml", "/etc/shadow")
                    .unwrap();
                b.into_inner().unwrap().finish().unwrap();
            }
            let dest = tmp.join(format!("out{i}"));
            std::fs::create_dir_all(&dest).unwrap();
            assert!(untar(&tar_path, &dest).is_err(), "{kind:?} 条目应被拒绝");
            assert!(std::fs::symlink_metadata(dest.join("app/docker-compose.yml")).is_err());
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn untar_rejects_path_traversal() {
        let tmp = std::env::temp_dir().join("flame-untar-test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        // 构造含 ../ 逃逸条目的 tar.gz（手工写 header，绕过 builder 的路径检查）
        let tar_path = tmp.join("evil.tar.gz");
        {
            use std::io::Write;
            let f = std::fs::File::create(&tar_path).unwrap();
            let mut enc = flate2::write::GzEncoder::new(f, flate2::Compression::default());
            // 直接写入原始 tar 头字节（name 含 ../）以构造恶意包（builder 会拒绝该路径）
            let mut raw: Vec<u8> = Vec::new();
            let mut h = [0u8; 512];
            let name = b"../escaped.txt";
            h[..name.len()].copy_from_slice(name);
            h[100..108].copy_from_slice(b"0000000\0"); // mode
            h[108..116].copy_from_slice(b"0000000\0"); // uid
            h[116..124].copy_from_slice(b"0000000\0"); // gid
            h[124..136].copy_from_slice(b"00000000004\0"); // size = 4 (octal 11+1)
            h[136..148].copy_from_slice(b"00000000000\0"); // mtime (11+1)
            h[148..156].copy_from_slice(b"        "); // checksum placeholder
            h[156] = b'0'; // type: regular file
            h[257..262].copy_from_slice(b"ustar");
            // 真实校验和
            let sum: u32 = h.iter().map(|&b| b as u32).sum();
            let cs = format!("{:06o}\0 ", sum);
            h[148..156].copy_from_slice(cs.as_bytes());
            raw.extend_from_slice(&h);
            raw.extend_from_slice(b"evil");
            raw.extend_from_slice(&[0u8; 508]); // padding to 1024 block
            enc.write_all(&raw).unwrap();
            enc.finish().unwrap();
        }
        let dest = tmp.join("out");
        std::fs::create_dir_all(&dest).unwrap();
        assert!(untar(&tar_path, &dest).is_err());
        assert!(!tmp.join("escaped.txt").exists(), "路径穿越被阻止");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn drill_down_single_dir_works() {
        let tmp = std::env::temp_dir().join("flame-drill-test");
        let _ = std::fs::remove_dir_all(&tmp);
        let wrapped = tmp.join("wrapped").join("gitea-1.21");
        std::fs::create_dir_all(&wrapped).unwrap();
        std::fs::write(wrapped.join("app.json"), b"{}").unwrap();
        let got = drill_down_single_dir(&tmp.join("wrapped"));
        assert_eq!(got, wrapped);
        // 多条目不下钻
        let multi = tmp.join("multi");
        std::fs::create_dir_all(multi.join("a")).unwrap();
        std::fs::create_dir_all(multi.join("b")).unwrap();
        assert_eq!(drill_down_single_dir(&multi), multi);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
