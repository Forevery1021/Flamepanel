//! WASM 插件宿主能力（capability）模型（Doc/21 §5.7，Doc/30 M-D）。
//!
//! 原则 **deny-by-default**：插件只能调用「基础宿主函数」（日志、读调用输入、写调用输出、
//! 读自身设置）；涉及宿主资源的函数必须持有管理员在安装时显式授予的能力：
//!
//! | 能力 | 允许 |
//! |------|------|
//! | `fs:read:<绝对目录>` | `flame.fs_read` / `flame.fs_list` 读该目录树 |
//! | `fs:write:<绝对目录>` | `flame.fs_write` 写该目录树（不含读） |
//! | `metrics:read` | `flame.metrics` 读面板本机最新指标快照 |
//! | `settings:write` | `flame.setting_set` 写自身 KV 设置 |
//!
//! 目录能力在**授予时**与**每次访问时**双重校验：授予时拒绝系统敏感根（`/`、`/proc`、
//! `/sys`、`/dev`、`/boot`、`/root`，写能力另拒 `/etc`、`/usr`、`/bin`…）；访问时目标路径
//! canonicalize（写目标 canonicalize 父目录）后必须仍在能力根之内，且不得命中凭据文件
//! 黑名单——根目录内指向 `/etc/shadow` 的符号链接同样被拒。

use crate::core::error::AppError;
use std::path::{Component, Path, PathBuf};

/// 单个插件可持有的能力数上限
pub const MAX_CAPABILITIES: usize = 16;

/// 读写均拒绝的根（及其子树）
const DENY_ALL_PREFIXES: &[&str] = &["/proc", "/sys", "/dev", "/boot", "/root"];
/// 写能力额外拒绝的根（系统目录：写入即提权 / 破坏系统）
const DENY_WRITE_PREFIXES: &[&str] = &[
    "/etc",
    "/usr",
    "/bin",
    "/sbin",
    "/lib",
    "/lib32",
    "/lib64",
    "/var/lib",
    "/var/spool",
];
/// 任何能力都不可读写的凭据文件 / 目录（访问时对 canonical 路径判定）
const DENY_FILES: &[&str] = &[
    "/etc/shadow",
    "/etc/gshadow",
    "/etc/shadow-",
    "/etc/gshadow-",
    "/etc/sudoers",
    "/etc/sudoers.d",
    "/etc/ssh",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capability {
    FsRead(PathBuf),
    FsWrite(PathBuf),
    MetricsRead,
    SettingsWrite,
}

impl Capability {
    pub fn parse(raw: &str) -> Result<Self, AppError> {
        let s = raw.trim();
        let bad = |why: &str| AppError::ValidationError(format!("capability {s:?}: {why}"));
        match s {
            "metrics:read" => return Ok(Self::MetricsRead),
            "settings:write" => return Ok(Self::SettingsWrite),
            _ => {}
        }
        let (write, dir) = if let Some(d) = s.strip_prefix("fs:read:") {
            (false, d)
        } else if let Some(d) = s.strip_prefix("fs:write:") {
            (true, d)
        } else {
            return Err(bad(
                "unknown capability (fs:read:<dir> | fs:write:<dir> | metrics:read | settings:write)",
            ));
        };
        let root = normalize_root(dir).map_err(|e| bad(&e))?;
        validate_root(&root, write).map_err(bad)?;
        // 已存在的能力根必须同时校验实际路径；尚未创建的安全目录仍可预先授予。
        if let Ok(canon) = root.canonicalize() {
            validate_root(&canon, write).map_err(bad)?;
        }
        Ok(if write {
            Self::FsWrite(root)
        } else {
            Self::FsRead(root)
        })
    }

    /// 规范字符串形式（存库 / 展示）
    pub fn as_string(&self) -> String {
        match self {
            Self::FsRead(p) => format!("fs:read:{}", p.display()),
            Self::FsWrite(p) => format!("fs:write:{}", p.display()),
            Self::MetricsRead => "metrics:read".into(),
            Self::SettingsWrite => "settings:write".into(),
        }
    }
}

/// 授予与访问共用同一份保护策略，避免 canonical 路径遗漏写保护目录。
fn protected_path(path: &Path, write: bool) -> bool {
    let extra: &[&str] = if write { DENY_WRITE_PREFIXES } else { &[] };
    DENY_ALL_PREFIXES
        .iter()
        .chain(extra)
        .chain(DENY_FILES)
        .any(|p| path.starts_with(p))
}

fn validate_root(root: &Path, write: bool) -> Result<(), &'static str> {
    if root == Path::new("/") || protected_path(root, write) {
        return Err("directory is not allowed for plugins");
    }
    // 祖先目录也不能把受保护子树间接纳入能力根。
    let ancestor_of = |list: &[&str]| list.iter().any(|p| Path::new(p).starts_with(root));
    if (write && ancestor_of(DENY_WRITE_PREFIXES)) || ancestor_of(DENY_ALL_PREFIXES) {
        return Err("directory contains protected system paths");
    }
    Ok(())
}

/// 绝对路径、拒绝 `..`（中间的 `.` 与重复 `/` 由 `Path::components` 规范化），去掉末尾 `/`。
fn normalize_root(dir: &str) -> Result<PathBuf, String> {
    if !dir.starts_with('/') {
        return Err("directory must be absolute".into());
    }
    if dir.len() > 4096 || dir.contains('\0') {
        return Err("invalid directory".into());
    }
    let mut out = PathBuf::from("/");
    for c in Path::new(dir).components() {
        match c {
            Component::RootDir => {}
            Component::Normal(seg) => out.push(seg),
            _ => return Err("directory must not contain '.' or '..'".into()),
        }
    }
    Ok(out)
}

/// 宿主函数访问被拒 / 失败的原因（映射到 ABI 负数错误码）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessError {
    Denied,
    Invalid,
    NotFound,
}

/// 一个插件的已授予能力集合。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Capabilities {
    caps: Vec<Capability>,
}

impl Capabilities {
    /// 解析并校验（去重、上限）。任一非法即整体拒绝。
    pub fn parse_all(raw: &[String]) -> Result<Self, AppError> {
        if raw.len() > MAX_CAPABILITIES {
            return Err(AppError::ValidationError(format!(
                "capabilities: at most {MAX_CAPABILITIES} entries"
            )));
        }
        let mut caps: Vec<Capability> = Vec::new();
        for r in raw {
            let c = Capability::parse(r)?;
            if !caps.contains(&c) {
                caps.push(c);
            }
        }
        Ok(Self { caps })
    }

    /// 已持久化的能力（授予时已校验）：解析失败的条目丢弃而非让插件整体不可用。
    pub fn from_stored(raw: &[String]) -> Self {
        let mut caps: Vec<Capability> = Vec::new();
        for c in raw.iter().filter_map(|r| Capability::parse(r).ok()) {
            if !caps.contains(&c) && caps.len() < MAX_CAPABILITIES {
                caps.push(c);
            }
        }
        Self { caps }
    }

    pub fn to_strings(&self) -> Vec<String> {
        self.caps.iter().map(Capability::as_string).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.caps.is_empty()
    }

    pub fn metrics_read(&self) -> bool {
        self.caps.contains(&Capability::MetricsRead)
    }

    pub fn settings_write(&self) -> bool {
        self.caps.contains(&Capability::SettingsWrite)
    }

    fn roots(&self, write: bool) -> impl Iterator<Item = &PathBuf> {
        self.caps.iter().filter_map(move |c| match (c, write) {
            (Capability::FsRead(p), false) | (Capability::FsWrite(p), true) => Some(p),
            _ => None,
        })
    }

    /// 解析读目标：相对路径基于第一个读能力根；canonicalize 后必须位于某个读能力根内。
    pub fn resolve_read(&self, path: &str) -> Result<PathBuf, AccessError> {
        let first = self.roots(false).next().ok_or(AccessError::Denied)?;
        let target = join_target(first, path)?;
        let canon = target.canonicalize().map_err(|_| AccessError::NotFound)?;
        self.check_within(&canon, false)?;
        Ok(canon)
    }

    /// 解析写目标：目标可能不存在，canonicalize **父目录**后拼文件名，再校验。
    pub fn resolve_write(&self, path: &str) -> Result<PathBuf, AccessError> {
        let first = self.roots(true).next().ok_or(AccessError::Denied)?;
        let target = join_target(first, path)?;
        let name = target.file_name().ok_or(AccessError::Invalid)?.to_owned();
        let parent = target.parent().ok_or(AccessError::Invalid)?;
        let canon_parent = parent.canonicalize().map_err(|_| AccessError::NotFound)?;
        let canon = canon_parent.join(name);
        // 目标已存在且是符号链接：按链接实际指向再判一次（防止经链接写出根目录）
        if let Ok(meta) = std::fs::symlink_metadata(&canon) {
            if meta.file_type().is_symlink() {
                let real = canon.canonicalize().map_err(|_| AccessError::Denied)?;
                self.check_within(&real, true)?;
            } else if meta.is_dir() {
                return Err(AccessError::Invalid);
            }
        }
        self.check_within(&canon, true)?;
        Ok(canon)
    }

    fn check_within(&self, canon: &Path, write: bool) -> Result<(), AccessError> {
        if protected_path(canon, write) {
            return Err(AccessError::Denied);
        }
        let inside = self.roots(write).any(|root| {
            // 根可能在授予后被重新定向；实际根也须重新满足授予策略（fail closed）。
            let Ok(root) = root.canonicalize() else {
                return false;
            };
            validate_root(&root, write).is_ok() && canon.starts_with(&root)
        });
        if inside {
            Ok(())
        } else {
            Err(AccessError::Denied)
        }
    }
}

fn join_target(first_root: &Path, path: &str) -> Result<PathBuf, AccessError> {
    if path.is_empty() || path.len() > 4096 || path.contains('\0') {
        return Err(AccessError::Invalid);
    }
    Ok(if path.starts_with('/') {
        PathBuf::from(path)
    } else {
        first_root.join(path)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps(v: &[&str]) -> Capabilities {
        Capabilities::parse_all(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap()
    }

    #[test]
    fn parse_accepts_known_and_normalizes() {
        assert_eq!(
            Capability::parse("metrics:read").unwrap(),
            Capability::MetricsRead
        );
        assert_eq!(
            Capability::parse("fs:read:/var/log/").unwrap(),
            Capability::FsRead("/var/log".into())
        );
        assert_eq!(
            Capability::parse("fs:write:/opt/flamepanel/plugins/x")
                .unwrap()
                .as_string(),
            "fs:write:/opt/flamepanel/plugins/x"
        );
        let c = caps(&["metrics:read", "metrics:read", "settings:write"]);
        assert_eq!(c.to_strings(), vec!["metrics:read", "settings:write"]);
    }

    #[test]
    fn parse_rejects_sensitive_or_malformed() {
        for bad in [
            "fs:read:/",
            "fs:read:/proc",
            "fs:read:/proc/1",
            "fs:read:/root/.ssh",
            "fs:read:/etc/shadow",
            "fs:read:/etc/ssh",
            "fs:write:/etc/nginx",
            "fs:write:/usr/local/bin",
            "fs:write:/var/lib/docker",
            "fs:write:/var",
            "fs:read:relative/dir",
            "fs:read:/var/log/../../etc",
            "net:connect:*",
            "exec",
        ] {
            assert!(Capability::parse(bad).is_err(), "{bad} 应被拒绝");
        }
        // 中间的 `.` 被规范化掉（等价路径，安全）
        assert_eq!(
            Capability::parse("fs:read:/var/./log").unwrap(),
            Capability::FsRead("/var/log".into())
        );
        // 读 /etc 下的普通配置允许（凭据文件在访问时拦截）
        assert!(Capability::parse("fs:read:/etc/nginx").is_ok());
        let many: Vec<String> = (0..17).map(|i| format!("fs:read:/tmp/d{i}")).collect();
        assert!(Capabilities::parse_all(&many).is_err());
    }

    #[test]
    fn resolve_read_and_write_stay_inside_roots() {
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("data");
        let outside = base.path().join("outside");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(root.join("sub/a.txt"), "hi").unwrap();
        std::fs::write(outside.join("secret"), "x").unwrap();
        let root_s = root.to_str().unwrap();
        let c = caps(&[
            &format!("fs:read:{root_s}"),
            &format!("fs:write:{root_s}/sub"),
        ]);

        assert!(c.resolve_read("sub/a.txt").is_ok(), "相对路径基于读根");
        assert!(c.resolve_read(&format!("{root_s}/sub/a.txt")).is_ok());
        assert_eq!(
            c.resolve_read(&format!("{root_s}/../outside/secret")),
            Err(AccessError::Denied),
            "`..` 逃逸"
        );
        assert_eq!(c.resolve_read("nope.txt"), Err(AccessError::NotFound));

        // 根内指向根外的符号链接
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(outside.join("secret"), root.join("link")).unwrap();
            assert_eq!(c.resolve_read("link"), Err(AccessError::Denied));
            std::os::unix::fs::symlink(&outside, root.join("sub/out")).unwrap();
            assert_eq!(
                c.resolve_write(&format!("{root_s}/sub/out/new")),
                Err(AccessError::Denied),
                "经目录链接写出根"
            );
            std::os::unix::fs::symlink(outside.join("secret"), root.join("sub/flink")).unwrap();
            assert_eq!(
                c.resolve_write(&format!("{root_s}/sub/flink")),
                Err(AccessError::Denied)
            );
        }

        assert!(c.resolve_write(&format!("{root_s}/sub/new.txt")).is_ok());
        assert_eq!(
            c.resolve_write(&format!("{root_s}/top.txt")),
            Err(AccessError::Denied),
            "写根是 sub，不含上层"
        );
        assert_eq!(
            c.resolve_write(&format!("{root_s}/sub")),
            Err(AccessError::Invalid),
            "不能把目录当文件写"
        );
        assert_eq!(c.resolve_read(""), Err(AccessError::Invalid));

        let none = Capabilities::default();
        assert_eq!(none.resolve_read("/tmp"), Err(AccessError::Denied));
        assert_eq!(none.resolve_write("/tmp/x"), Err(AccessError::Denied));
        assert!(!none.metrics_read() && !none.settings_write());
    }

    #[test]
    fn credential_files_denied_even_inside_granted_root() {
        let c = caps(&["fs:read:/etc"]);
        if Path::new("/etc/shadow").exists() {
            assert_eq!(c.resolve_read("/etc/shadow"), Err(AccessError::Denied));
        }
        assert!(c.resolve_read("/etc/ssh/sshd_config").is_err());
        assert!(c.resolve_read("/etc/hostname").is_ok() || !Path::new("/etc/hostname").exists());
    }

    #[cfg(unix)]
    #[test]
    fn parse_rejects_symlinked_protected_capability_roots() {
        let base = tempfile::tempdir().unwrap();
        for (index, (mode, destination)) in [("write", "/etc"), ("write", "/usr"), ("read", "/dev")]
            .into_iter()
            .enumerate()
        {
            let alias = base.path().join(format!("alias-{index}"));
            std::os::unix::fs::symlink(destination, &alias).unwrap();
            let raw = format!("fs:{mode}:{}", alias.display());
            assert!(Capability::parse(&raw).is_err(), "{raw} -> {destination}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn resolve_write_rechecks_protection_after_root_symlink_is_retargeted() {
        let base = tempfile::tempdir().unwrap();
        let data = base.path().join("data");
        let alias = base.path().join("granted");
        std::fs::create_dir(&data).unwrap();
        std::os::unix::fs::symlink(&data, &alias).unwrap();
        let c = caps(&[&format!("fs:write:{}", alias.display())]);
        assert!(c.resolve_write("new.txt").is_ok());
        std::fs::remove_file(&alias).unwrap();
        std::os::unix::fs::symlink("/etc", &alias).unwrap();
        assert_eq!(c.resolve_write("passwd"), Err(AccessError::Denied));
    }

    #[cfg(unix)]
    #[test]
    fn from_stored_drops_protected_root_aliases() {
        let base = tempfile::tempdir().unwrap();
        let alias = base.path().join("protected");
        std::os::unix::fs::symlink("/etc", &alias).unwrap();
        let c = Capabilities::from_stored(&[format!("fs:write:{}", alias.display())]);
        assert!(c.to_strings().is_empty());
        assert_eq!(c.resolve_write("passwd"), Err(AccessError::Denied));
    }

    #[cfg(unix)]
    #[test]
    fn safe_symlinked_capability_roots_remain_usable() {
        let base = tempfile::tempdir().unwrap();
        let data = base.path().join("data");
        let alias = base.path().join("granted");
        std::fs::create_dir(&data).unwrap();
        std::fs::write(data.join("file.txt"), "safe").unwrap();
        std::os::unix::fs::symlink(&data, &alias).unwrap();
        let c = caps(&[
            &format!("fs:read:{}", alias.display()),
            &format!("fs:write:{}", alias.display()),
        ]);
        assert_eq!(c.resolve_read("file.txt").unwrap(), data.join("file.txt"));
        assert_eq!(c.resolve_write("new.txt").unwrap(), data.join("new.txt"));
        let future = base.path().join("not-created-yet");
        assert!(Capability::parse(&format!("fs:write:{}", future.display())).is_ok());
    }

    #[test]
    fn from_stored_drops_invalid_entries() {
        let c = Capabilities::from_stored(&["metrics:read".into(), "fs:read:/".into()]);
        assert_eq!(c.to_strings(), vec!["metrics:read"]);
    }
}
