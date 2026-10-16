//! 主机交互：systemd、root 判定、磁盘与目录占用、原子写文件。

use std::io::Write;
use std::path::Path;
use std::process::Command;

use super::context::SERVICE_NAME;
use super::output::CliError;

pub fn is_root() -> bool {
    // SAFETY: geteuid 无副作用、总是成功
    unsafe { libc::geteuid() == 0 }
}

/// 主机由 systemd 管理（容器内通常为否）。
pub fn has_systemd() -> bool {
    Path::new("/run/systemd/system").is_dir() && which("systemctl")
}

pub fn which(bin: &str) -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join(bin).is_file()))
        .unwrap_or(false)
}

/// 执行命令，返回 (是否成功, stdout, stderr)。命令不存在视为失败。
pub fn run(cmd: &str, args: &[&str]) -> (bool, String, String) {
    match Command::new(cmd).args(args).output() {
        Ok(o) => (
            o.status.success(),
            String::from_utf8_lossy(&o.stdout).trim().to_string(),
            String::from_utf8_lossy(&o.stderr).trim().to_string(),
        ),
        Err(e) => (false, String::new(), e.to_string()),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ServiceState {
    pub manager: &'static str,
    /// `active` / `inactive` / `failed` / `activating` …；无 systemd 时为 `unknown`
    pub state: String,
    pub enabled: Option<bool>,
}

pub fn service_state() -> ServiceState {
    if !has_systemd() {
        return ServiceState {
            manager: "none",
            state: "unknown".into(),
            enabled: None,
        };
    }
    let (_, enabled, enabled_err) = run("systemctl", &["is-enabled", SERVICE_NAME]);
    // 有 systemd 但没装单元（手动运行 / 开发机）：不要误报成 inactive
    if enabled.is_empty() || enabled == "not-found" || enabled_err.contains("No such file") {
        return ServiceState {
            manager: "systemd",
            state: "not-installed".into(),
            enabled: None,
        };
    }
    let (_, state, _) = run("systemctl", &["is-active", SERVICE_NAME]);
    ServiceState {
        manager: "systemd",
        state: if state.is_empty() {
            "unknown".into()
        } else {
            state
        },
        enabled: Some(enabled == "enabled"),
    }
}

/// `systemctl <verb> flamepanel`；失败时附带排障提示。
/// 本机特权助手 systemd 单元（install.sh 生成）。
pub const HELPER_SERVICE_NAME: &str = "flamepanel-helper";

pub fn systemctl(verb: &str) -> Result<(), CliError> {
    if !has_systemd() {
        return Err(CliError::general(
            "systemd is not available on this host (in Docker use `docker restart <container>`)",
        ));
    }
    if verb == "restart" {
        // 升级 / 回滚 / 重启后特权助手也要换到同一二进制（线协议随版本演进，Doc/32 W3-b）；
        // try-restart 对未安装 / 未运行的助手是空操作，失败不阻断面板重启
        let _ = run("systemctl", &["try-restart", HELPER_SERVICE_NAME]);
    }
    let (ok, _, err) = run("systemctl", &[verb, SERVICE_NAME]);
    if ok {
        Ok(())
    } else {
        Err(CliError::general(format!(
            "systemctl {verb} {SERVICE_NAME} failed: {err}\n  hint: journalctl -u {SERVICE_NAME} -n 50 --no-pager; fpctl logs --level error -n 20"
        )))
    }
}

/// 目录递归占用（不跟随符号链接，读不到的条目忽略）。
pub fn dir_size(path: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    if meta.is_file() {
        return meta.len();
    }
    if !meta.is_dir() {
        return 0;
    }
    let mut total = 0;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let Ok(m) = e.metadata() else { continue };
            if m.is_dir() {
                stack.push(e.path());
            } else if m.is_file() {
                total += m.len();
            }
        }
    }
    total
}

/// (可用字节, 总字节)
pub fn disk_space(path: &Path) -> Option<(u64, u64)> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    // SAFETY: statvfs 写入调用方提供的结构体；路径为合法 C 字符串
    unsafe {
        let mut s: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(c.as_ptr(), &mut s) != 0 {
            return None;
        }
        let frag = s.f_frsize as u64;
        Some((s.f_bavail as u64 * frag, s.f_blocks as u64 * frag))
    }
}

/// 原子替换文件内容：写同目录临时文件 → fsync → 保留原权限与属主 → rename。
pub fn write_atomic(path: &Path, content: &str) -> Result<(), CliError> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let dir = path.parent().unwrap_or(Path::new("."));
    let old = std::fs::metadata(path).ok();
    let mut tmp = tempfile::NamedTempFile::new_in(dir)
        .map_err(|e| CliError::io(format!("create temp file in {}", dir.display()), &e))?;
    tmp.write_all(content.as_bytes())
        .and_then(|_| tmp.as_file().sync_all())
        .map_err(|e| CliError::io(format!("write {}", path.display()), &e))?;
    if let Some(m) = &old {
        let _ = std::fs::set_permissions(
            tmp.path(),
            std::fs::Permissions::from_mode(m.mode() & 0o7777),
        );
        let _ = std::os::unix::fs::chown(tmp.path(), Some(m.uid()), Some(m.gid()));
    }
    tmp.persist(path)
        .map_err(|e| CliError::io(format!("replace {}", path.display()), &e.error))?;
    Ok(())
}

pub fn human_bytes(n: u64) -> String {
    const U: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{n} B")
    } else {
        format!("{v:.1} {}", U[i])
    }
}

pub fn human_duration(secs: u64) -> String {
    let (d, h, m) = (secs / 86400, secs % 86400 / 3600, secs % 3600 / 60);
    match (d, h) {
        (0, 0) => format!("{m}m{}s", secs % 60),
        (0, _) => format!("{h}h{m}m"),
        _ => format!("{d}d{h}h"),
    }
}

/// 简单对齐表格。
pub fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut w: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();
    for r in rows {
        for (i, c) in r.iter().enumerate() {
            if i < w.len() {
                w[i] = w[i].max(c.chars().count());
            }
        }
    }
    let line = |cells: Vec<String>| -> String {
        let mut s = String::new();
        for (i, c) in cells.iter().enumerate() {
            if i + 1 == cells.len() {
                s.push_str(c);
            } else {
                let pad = w[i].saturating_sub(c.chars().count());
                s.push_str(c);
                s.push_str(&" ".repeat(pad + 2));
            }
        }
        s.trim_end().to_string()
    };
    let mut out = line(headers.iter().map(|h| h.to_string()).collect());
    out.push('\n');
    for r in rows {
        out.push_str(&line(r.clone()));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting_helpers() {
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1536), "1.5 KiB");
        assert_eq!(human_bytes(3 * 1024 * 1024 * 1024), "3.0 GiB");
        assert_eq!(human_duration(59), "0m59s");
        assert_eq!(human_duration(3 * 3600 + 120), "3h2m");
        assert_eq!(human_duration(2 * 86400 + 3600), "2d1h");
        let t = table(&["ID", "NAME"], &[vec!["1".into(), "admin".into()]]);
        assert_eq!(t, "ID  NAME\n1   admin\n");
    }

    #[test]
    fn dir_size_and_disk_space() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a"), vec![0u8; 100]).unwrap();
        std::fs::create_dir(tmp.path().join("sub")).unwrap();
        std::fs::write(tmp.path().join("sub/b"), vec![0u8; 50]).unwrap();
        assert_eq!(dir_size(tmp.path()), 150);
        assert_eq!(dir_size(&tmp.path().join("missing")), 0);
        let (avail, total) = disk_space(tmp.path()).unwrap();
        assert!(total > 0 && avail <= total);
    }

    #[test]
    fn write_atomic_preserves_mode() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("f.env");
        std::fs::write(&p, "A=1\n").unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600)).unwrap();
        write_atomic(&p, "A=2\n").unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "A=2\n");
        let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
