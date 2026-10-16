//! 版本化安装目录（Doc/32 §5）。
//!
//! ```text
//! <home>/releases/<版本>/bin/{flamepanel,fpctl}
//! <home>/releases/<版本>/frontend/dist/
//! <home>/releases/.previous          上一次切走的版本（rollback 目标）
//! <home>/current  -> releases/<版本>  （相对符号链接，原子替换）
//! <home>/frontend -> current/frontend （nginx `root <home>/frontend/dist` 不用改）
//! <bin>/flamepanel -> <home>/current/bin/flamepanel  （systemd ExecStart 不用改）
//! <bin>/fpctl      -> <home>/current/bin/fpctl
//! ```
//!
//! 数据（`data/`、`logs/`、env 文件）始终在 `<home>` 下，与版本无关；切换版本只动符号链接。

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::output::CliError;

pub const DEFAULT_BIN_DIR: &str = "/usr/local/bin";
pub const BINARIES: [&str; 2] = ["flamepanel", "fpctl"];
const PREVIOUS_FILE: &str = ".previous";

/// 语义化版本比较键：`1.10.0` > `1.9.0`；预发布（`-rc1`）低于正式版；无法解析的排最后。
pub fn version_key(v: &str) -> (Vec<u64>, u8, String) {
    let (core, pre) = match v.split_once('-') {
        Some((c, p)) => (c, p.to_string()),
        None => (v, String::new()),
    };
    let nums: Option<Vec<u64>> = core.split('.').map(|p| p.parse().ok()).collect();
    match nums {
        Some(n) => (n, if pre.is_empty() { 1 } else { 0 }, pre),
        None => (Vec::new(), 0, v.to_string()),
    }
}

/// 版本目录名白名单：防止 `..` / 路径分隔符。
pub fn valid_version(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 64
        && !v.starts_with('.')
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'))
}

/// 从 `flamepanel --version` 的输出解析版本（`flamepanel 1.2.3`）。
pub fn parse_version_output(out: &str) -> Option<String> {
    let v = out.split_whitespace().last()?.trim_start_matches('v');
    valid_version(v).then(|| v.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LayoutState {
    /// `current` 符号链接存在
    Versioned,
    /// 二进制是 `<bin>/flamepanel` 普通文件（install.sh 旧布局）
    Legacy,
    /// 什么都没有
    Empty,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReleaseInfo {
    pub version: String,
    pub current: bool,
    pub previous: bool,
    pub path: PathBuf,
    pub has_frontend: bool,
    pub has_fpctl: bool,
    pub bytes: u64,
}

pub struct Layout {
    pub home: PathBuf,
    pub bin_dir: PathBuf,
}

fn is_symlink(p: &Path) -> bool {
    std::fs::symlink_metadata(p)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

fn exists_no_follow(p: &Path) -> bool {
    std::fs::symlink_metadata(p).is_ok()
}

/// 原子地把 `link` 指向 `target`：先建临时链接再 rename 覆盖。
pub fn atomic_symlink(target: &Path, link: &Path) -> Result<(), CliError> {
    let dir = link.parent().unwrap_or(Path::new("."));
    let name = link
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = dir.join(format!(".{name}.tmp-{}", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    std::os::unix::fs::symlink(target, &tmp)
        .map_err(|e| CliError::io(format!("symlink {}", tmp.display()), &e))?;
    std::fs::rename(&tmp, link).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        CliError::io(format!("replace {}", link.display()), &e)
    })
}

impl Layout {
    pub fn new(home: &Path, bin_dir: &Path) -> Self {
        Self {
            home: home.to_path_buf(),
            bin_dir: bin_dir.to_path_buf(),
        }
    }

    pub fn releases_dir(&self) -> PathBuf {
        self.home.join("releases")
    }
    pub fn release_dir(&self, v: &str) -> PathBuf {
        self.releases_dir().join(v)
    }
    fn current_link(&self) -> PathBuf {
        self.home.join("current")
    }
    fn frontend_link(&self) -> PathBuf {
        self.home.join("frontend")
    }

    pub fn state(&self) -> LayoutState {
        if is_symlink(&self.current_link()) {
            LayoutState::Versioned
        } else if std::fs::symlink_metadata(self.bin_dir.join("flamepanel"))
            .map(|m| m.is_file())
            .unwrap_or(false)
        {
            LayoutState::Legacy
        } else {
            LayoutState::Empty
        }
    }

    pub fn current_version(&self) -> Option<String> {
        let target = std::fs::read_link(self.current_link()).ok()?;
        target.file_name().map(|n| n.to_string_lossy().into_owned())
    }

    pub fn previous_version(&self) -> Option<String> {
        let recorded = std::fs::read_to_string(self.releases_dir().join(PREVIOUS_FILE))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|v| valid_version(v) && self.release_dir(v).is_dir());
        let current = self.current_version();
        if recorded.is_some() && recorded != current {
            return recorded;
        }
        // 无记录：取低于当前版本的最高版本
        let cur_key = current.as_deref().map(version_key);
        self.versions()
            .into_iter()
            .filter(|v| Some(v) != current.as_ref())
            .filter(|v| cur_key.as_ref().is_none_or(|k| version_key(v) < *k))
            .max_by_key(|v| version_key(v))
    }

    /// 已安装版本（按版本号升序）。
    pub fn versions(&self) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(self.releases_dir())
            .map(|rd| {
                rd.flatten()
                    .filter(|e| e.path().is_dir() && !is_symlink(&e.path()))
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .filter(|n| valid_version(n))
                    .collect()
            })
            .unwrap_or_default();
        v.sort_by_key(|x| version_key(x));
        v
    }

    pub fn list(&self) -> Vec<ReleaseInfo> {
        let current = self.current_version();
        let previous = self.previous_version();
        let mut out: Vec<ReleaseInfo> = self
            .versions()
            .into_iter()
            .map(|v| {
                let path = self.release_dir(&v);
                ReleaseInfo {
                    current: current.as_deref() == Some(v.as_str()),
                    previous: previous.as_deref() == Some(v.as_str()),
                    has_frontend: path.join("frontend/dist/index.html").is_file(),
                    has_fpctl: path.join("bin/fpctl").is_file(),
                    bytes: super::system::dir_size(&path),
                    path,
                    version: v,
                }
            })
            .collect();
        out.reverse(); // 新版本在前
        out
    }

    /// 稳定路径 → `current` 的链接：`(链接路径, 链接目标)`。
    fn stable_links(&self) -> Vec<(PathBuf, PathBuf)> {
        let cur = self.current_link();
        let mut v: Vec<(PathBuf, PathBuf)> = BINARIES
            .iter()
            .map(|b| (self.bin_dir.join(b), cur.join("bin").join(b)))
            .collect();
        v.push((self.frontend_link(), PathBuf::from("current/frontend")));
        v
    }

    /// 让稳定路径都指向 `current`；遇到普通文件/目录（未收编的旧布局）拒绝覆盖。
    pub fn ensure_links(&self) -> Result<Vec<String>, CliError> {
        let mut done = Vec::new();
        for (link, target) in self.stable_links() {
            if exists_no_follow(&link) && !is_symlink(&link) {
                return Err(CliError::general(format!(
                    "{} is a regular file/dir; run `fpctl adopt` first",
                    link.display()
                )));
            }
            if std::fs::read_link(&link).ok().as_deref() != Some(target.as_path()) {
                atomic_symlink(&target, &link)?;
                done.push(format!("{} -> {}", link.display(), target.display()));
            }
        }
        Ok(done)
    }

    /// 切换 `current` 到指定版本并记录上一个版本。
    pub fn switch(&self, v: &str) -> Result<Option<String>, CliError> {
        if !valid_version(v) || !self.release_dir(v).join("bin/flamepanel").is_file() {
            return Err(CliError::not_found(format!(
                "release {v} is not installed (see `fpctl releases`)"
            )));
        }
        let before = self.current_version();
        atomic_symlink(&PathBuf::from("releases").join(v), &self.current_link())?;
        if let Some(b) = &before {
            if b != v {
                let _ = std::fs::write(self.releases_dir().join(PREVIOUS_FILE), format!("{b}\n"));
            }
        }
        self.ensure_links()?;
        Ok(before)
    }

    /// 旧布局收编：把 `<bin>/flamepanel|fpctl` 与 `<home>/frontend` 移入 `releases/<v>`，再建链接。
    /// 幂等：已是版本化布局时只补链接。
    pub fn adopt(&self, v: &str) -> Result<Vec<String>, CliError> {
        if !valid_version(v) {
            return Err(CliError::usage(format!("invalid version '{v}'")));
        }
        let mut actions = Vec::new();
        let dest = self.release_dir(v);
        let legacy_bin = self.bin_dir.join("flamepanel");
        let has_legacy_files = std::fs::symlink_metadata(&legacy_bin)
            .map(|m| m.is_file())
            .unwrap_or(false);
        if !has_legacy_files && self.state() == LayoutState::Versioned {
            actions.extend(self.ensure_links()?);
            return Ok(actions);
        }
        if !has_legacy_files {
            return Err(CliError::not_found(format!(
                "nothing to adopt: {} not found",
                legacy_bin.display()
            )));
        }
        // 同版本目录已存在（重装同版本）：旧目录让位
        if dest.exists() {
            let aside = self
                .releases_dir()
                .join(format!(".{v}.replaced-{}", chrono::Utc::now().timestamp()));
            std::fs::rename(&dest, &aside)
                .map_err(|e| CliError::io(format!("move aside {}", dest.display()), &e))?;
            actions.push(format!("moved existing {} aside", dest.display()));
        }
        std::fs::create_dir_all(dest.join("bin"))
            .map_err(|e| CliError::io(format!("create {}", dest.display()), &e))?;
        for b in BINARIES {
            let src = self.bin_dir.join(b);
            if std::fs::symlink_metadata(&src)
                .map(|m| m.is_file())
                .unwrap_or(false)
            {
                move_path(&src, &dest.join("bin").join(b))?;
                actions.push(format!(
                    "{} -> {}",
                    src.display(),
                    dest.join("bin").join(b).display()
                ));
            }
        }
        let fe = self.frontend_link();
        if fe.is_dir() && !is_symlink(&fe) {
            move_path(&fe, &dest.join("frontend"))?;
            actions.push(format!(
                "{} -> {}",
                fe.display(),
                dest.join("frontend").display()
            ));
        }
        // 清理残留的 .replaced 目录
        if let Ok(rd) = std::fs::read_dir(self.releases_dir()) {
            for e in rd.flatten() {
                if e.file_name().to_string_lossy().contains(".replaced-") {
                    let _ = std::fs::remove_dir_all(e.path());
                }
            }
        }
        self.switch(v)?;
        actions.push(format!("current -> releases/{v}"));
        Ok(actions)
    }

    /// 保留最新 `keep` 个版本，当前与上一个版本永不删除。返回被删除的版本。
    pub fn prune(&self, keep: usize) -> Vec<String> {
        let current = self.current_version();
        let previous = self.previous_version();
        let versions = self.versions(); // 升序
        let excess = versions.len().saturating_sub(keep);
        let mut removed = Vec::new();
        for v in versions.into_iter().take(excess) {
            if Some(&v) == current.as_ref() || Some(&v) == previous.as_ref() {
                continue;
            }
            if std::fs::remove_dir_all(self.release_dir(&v)).is_ok() {
                removed.push(v);
            }
        }
        removed
    }
}

/// rename，跨文件系统时退化为复制 + 删除。
pub fn move_path(src: &Path, dst: &Path) -> Result<(), CliError> {
    if let Some(p) = dst.parent() {
        std::fs::create_dir_all(p).map_err(|e| CliError::io(p.display(), &e))?;
    }
    match std::fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(e) if e.raw_os_error() == Some(libc::EXDEV) => {
            copy_recursive(src, dst)?;
            if src.is_dir() {
                std::fs::remove_dir_all(src)
            } else {
                std::fs::remove_file(src)
            }
            .map_err(|e| CliError::io(src.display(), &e))
        }
        Err(e) => Err(CliError::io(format!("move {}", src.display()), &e)),
    }
}

fn copy_recursive(src: &Path, dst: &Path) -> Result<(), CliError> {
    if src.is_dir() {
        std::fs::create_dir_all(dst).map_err(|e| CliError::io(dst.display(), &e))?;
        for e in std::fs::read_dir(src)
            .map_err(|e| CliError::io(src.display(), &e))?
            .flatten()
        {
            copy_recursive(&e.path(), &dst.join(e.file_name()))?;
        }
        Ok(())
    } else {
        std::fs::copy(src, dst)
            .map(|_| ())
            .map_err(|e| CliError::io(format!("copy {}", src.display()), &e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_ordering_and_validation() {
        let mut v = vec!["1.10.0", "1.2.0", "1.2.0-rc1", "1.9.3", "weird"];
        v.sort_by_key(|x| version_key(x));
        assert_eq!(v, vec!["weird", "1.2.0-rc1", "1.2.0", "1.9.3", "1.10.0"]);
        assert!(valid_version("1.2.3-rc.1+build5"));
        for bad in ["", "..", "../x", "a/b", ".hidden", "1 2"] {
            assert!(!valid_version(bad), "{bad}");
        }
        assert_eq!(
            parse_version_output("flamepanel 1.2.3\n").as_deref(),
            Some("1.2.3")
        );
        assert_eq!(parse_version_output("v1.0.0").as_deref(), Some("1.0.0"));
        assert_eq!(parse_version_output("").as_deref(), None);
    }

    fn legacy(tmp: &Path) -> Layout {
        let home = tmp.join("home");
        let bin = tmp.join("bin");
        std::fs::create_dir_all(home.join("frontend/dist")).unwrap();
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(home.join("frontend/dist/index.html"), "v1").unwrap();
        std::fs::write(bin.join("flamepanel"), "bin v1").unwrap();
        std::fs::write(bin.join("fpctl"), "cli v1").unwrap();
        Layout::new(&home, &bin)
    }

    fn fake_release(l: &Layout, v: &str) {
        let d = l.release_dir(v);
        std::fs::create_dir_all(d.join("bin")).unwrap();
        std::fs::create_dir_all(d.join("frontend/dist")).unwrap();
        std::fs::write(d.join("bin/flamepanel"), format!("bin {v}")).unwrap();
        std::fs::write(d.join("bin/fpctl"), format!("cli {v}")).unwrap();
        std::fs::write(d.join("frontend/dist/index.html"), v).unwrap();
    }

    #[test]
    fn adopt_switch_rollback_prune() {
        let tmp = tempfile::tempdir().unwrap();
        let l = legacy(tmp.path());
        assert_eq!(l.state(), LayoutState::Legacy);

        l.adopt("1.0.0").unwrap();
        assert_eq!(l.state(), LayoutState::Versioned);
        assert_eq!(l.current_version().as_deref(), Some("1.0.0"));
        // 稳定路径经符号链接读到同样内容
        assert_eq!(
            std::fs::read_to_string(l.bin_dir.join("flamepanel")).unwrap(),
            "bin v1"
        );
        assert_eq!(
            std::fs::read_to_string(l.home.join("frontend/dist/index.html")).unwrap(),
            "v1"
        );
        // 幂等
        assert!(l.adopt("1.0.0").unwrap().is_empty());

        fake_release(&l, "1.1.0");
        assert_eq!(l.switch("1.1.0").unwrap().as_deref(), Some("1.0.0"));
        assert_eq!(
            std::fs::read_to_string(l.bin_dir.join("fpctl")).unwrap(),
            "cli 1.1.0"
        );
        assert_eq!(l.previous_version().as_deref(), Some("1.0.0"));

        // 回滚目标 = 记录的上一个版本
        l.switch("1.0.0").unwrap();
        assert_eq!(l.previous_version().as_deref(), Some("1.1.0"));
        l.switch("1.1.0").unwrap();

        assert!(l.switch("9.9.9").is_err());
        assert!(l.switch("../x").is_err());

        fake_release(&l, "1.2.0");
        fake_release(&l, "1.3.0");
        l.switch("1.3.0").unwrap(); // previous = 1.1.0
        let removed = l.prune(2);
        // 1.0.0 被删；1.1.0 虽超额但是 previous，保留
        assert_eq!(removed, vec!["1.0.0"]);
        let vs: Vec<String> = l.list().into_iter().map(|r| r.version).collect();
        assert_eq!(vs, vec!["1.3.0", "1.2.0", "1.1.0"]);
        let cur = l.list().into_iter().find(|r| r.current).unwrap();
        assert_eq!(cur.version, "1.3.0");
    }

    #[test]
    fn readopt_after_reinstall_replaces_release_and_refuses_unadopted_links() {
        let tmp = tempfile::tempdir().unwrap();
        let l = legacy(tmp.path());
        l.adopt("1.0.0").unwrap();
        // install.sh 重跑：移除稳定路径链接后重新平铺部署同版本
        for b in BINARIES {
            std::fs::remove_file(l.bin_dir.join(b)).unwrap();
        }
        std::fs::remove_file(l.home.join("frontend")).unwrap();
        std::fs::write(l.bin_dir.join("flamepanel"), "bin v1 rebuilt").unwrap();
        // 仅有二进制、尚未收编时 ensure_links 拒绝覆盖普通文件
        assert!(l.ensure_links().is_err());
        std::fs::create_dir_all(l.home.join("frontend/dist")).unwrap();
        std::fs::write(l.home.join("frontend/dist/index.html"), "v1b").unwrap();
        l.adopt("1.0.0").unwrap();
        assert_eq!(
            std::fs::read_to_string(l.bin_dir.join("flamepanel")).unwrap(),
            "bin v1 rebuilt"
        );
        assert_eq!(l.versions(), vec!["1.0.0"]);
        // 无 fpctl 的新布局：fpctl 链接悬空但不报错
        assert!(!l.release_dir("1.0.0").join("bin/fpctl").exists());
    }
}
