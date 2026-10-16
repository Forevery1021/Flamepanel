//! Node.js 项目部署约定（S-11 · Doc/20 §5.3）。
//!
//! 与 Python 的 venv 约定（[`super::python`]）同构，部署后由 `ProjectService` 编排：
//! - **依赖安装**：项目根有 `package.json` 时，按「`package.json` + 锁文件」摘要判定是否需要
//!   重装（标记 `node_modules/.fp-deps-stamp`）；有 `package-lock.json` / `npm-shrinkwrap.json`
//!   用 `npm ci`（可复现），否则 `npm install`；声明了 `scripts.build` 则随后 `npm run build`；
//! - **启动命令探测**（仅在用户与 Procfile 都未给出时回填）：
//!   `ecosystem.config.{js,cjs}` → `pm2-runtime start …`；`scripts.start` → `npm start`；
//!   `main` 字段 → `node <main>`；再依次找 `server.js` / `app.js` / `index.js` / `main.js`。
//!
//! 回填的命令是裸 `node` / `npm` / `pm2-runtime`，由 [`crate::domain::project_command`]
//! 在交给守护器前补全到绑定运行时的 `bin/`。

use crate::application::execution_mode::PrivilegedCommand;
use std::path::Path;

/// 依赖安装标记文件（位于 `node_modules/` 下，删掉 node_modules 自然触发重装）。
pub const DEPS_STAMP: &str = "node_modules/.fp-deps-stamp";
/// npm 安装 / 构建超时（秒）。
pub const NPM_TIMEOUT_SECS: u64 = 1800;

const LOCKFILES: &[&str] = &["package-lock.json", "npm-shrinkwrap.json"];
const ECOSYSTEM_FILES: &[&str] = &["ecosystem.config.js", "ecosystem.config.cjs"];
const ENTRY_CANDIDATES: &[&str] = &["server.js", "app.js", "index.js", "main.js"];

fn read_package_json(deploy: &Path) -> Option<serde_json::Value> {
    let raw = std::fs::read_to_string(deploy.join("package.json")).ok()?;
    serde_json::from_str(&raw).ok()
}

/// 项目根是否有 `package.json`。
pub fn has_package_json(deploy: &Path) -> bool {
    deploy.join("package.json").is_file()
}

fn has_script(pkg: &serde_json::Value, name: &str) -> bool {
    pkg.get("scripts")
        .and_then(|s| s.get(name))
        .and_then(|v| v.as_str())
        .is_some_and(|v| !v.trim().is_empty())
}

/// 启动命令探测（见模块文档的优先级）；探测不到返回 None。
pub fn detect_start_command(deploy: &Path) -> Option<String> {
    if let Some(eco) = ECOSYSTEM_FILES.iter().find(|f| deploy.join(f).is_file()) {
        return Some(format!("pm2-runtime start {eco}"));
    }
    let pkg = read_package_json(deploy);
    if let Some(pkg) = &pkg {
        if has_script(pkg, "start") {
            return Some("npm start".into());
        }
        if let Some(main) = pkg.get("main").and_then(|m| m.as_str()) {
            let main = main.trim().trim_start_matches("./");
            // 只接受安全的相对路径（守护器禁止 shell 元字符，这里提前挡掉空白与穿越）
            let safe = !main.is_empty()
                && !main.starts_with('/')
                && !main.split('/').any(|seg| seg == "..")
                && main
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "._-/@".contains(c));
            if safe && deploy.join(main).is_file() {
                return Some(format!("node {main}"));
            }
        }
    }
    ENTRY_CANDIDATES
        .iter()
        .find(|f| deploy.join(f).is_file())
        .map(|f| format!("node {f}"))
}

/// 依赖摘要：`package.json` + 锁文件内容（任一变化即重装）。
pub fn deps_digest(deploy: &Path) -> Option<String> {
    let mut buf = std::fs::read(deploy.join("package.json")).ok()?;
    for lock in LOCKFILES {
        if let Ok(bytes) = std::fs::read(deploy.join(lock)) {
            buf.extend_from_slice(b"\0");
            buf.extend_from_slice(lock.as_bytes());
            buf.extend_from_slice(&bytes);
        }
    }
    Some(super::download::content_digest(&buf))
}

/// 依赖是否需要（重新）安装：标记内容与当前摘要不一致。
pub fn deps_outdated(deploy: &Path) -> bool {
    match deps_digest(deploy) {
        Some(d) => {
            std::fs::read_to_string(deploy.join(DEPS_STAMP))
                .ok()
                .as_deref()
                != Some(&d)
        }
        None => false,
    }
}

/// 写入依赖标记。
pub fn write_deps_stamp(deploy: &Path) {
    if let Some(d) = deps_digest(deploy) {
        let _ = std::fs::write(deploy.join(DEPS_STAMP), d);
    }
}

/// `npm` 程序路径：绑定运行时用 `<bin>/npm`，否则系统 `npm`。
fn npm_program(bin: Option<&str>) -> String {
    bin.map(|b| format!("{}/npm", b.trim_end_matches('/')))
        .unwrap_or_else(|| "npm".into())
}

fn npm_command(bin: Option<&str>, deploy: &Path, args: &[&str]) -> PrivilegedCommand {
    let mut cmd = PrivilegedCommand::new(
        npm_program(bin),
        args.iter().map(|a| a.to_string()).collect(),
    )
    .cwd(deploy.to_string_lossy())
    .timeout(NPM_TIMEOUT_SECS);
    if let Some(b) = bin {
        cmd = cmd.env("PATH", super::node::path_with_bin(b.trim_end_matches('/')));
    }
    cmd
}

/// 依赖安装命令：有锁文件 `npm ci`，否则 `npm install`。
pub fn install_command(bin: Option<&str>, deploy: &Path) -> PrivilegedCommand {
    let has_lock = LOCKFILES.iter().any(|l| deploy.join(l).is_file());
    let args: &[&str] = if has_lock {
        &["ci", "--no-audit", "--no-fund"]
    } else {
        &["install", "--no-audit", "--no-fund"]
    };
    npm_command(bin, deploy, args)
}

/// 构建命令（仅当声明了 `scripts.build`）。
pub fn build_command(bin: Option<&str>, deploy: &Path) -> Option<PrivilegedCommand> {
    let pkg = read_package_json(deploy)?;
    has_script(&pkg, "build").then(|| npm_command(bin, deploy, &["run", "build"]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn project(files: &[(&str, &str)]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fp-nodeproj-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        for (name, content) in files {
            let p = dir.join(name);
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(p, content).unwrap();
        }
        dir
    }

    #[test]
    fn start_detection_priority() {
        let d = project(&[
            ("ecosystem.config.js", "module.exports={}"),
            ("package.json", r#"{"scripts":{"start":"node x"}}"#),
            ("server.js", ""),
        ]);
        assert_eq!(
            detect_start_command(&d).as_deref(),
            Some("pm2-runtime start ecosystem.config.js")
        );
        std::fs::remove_file(d.join("ecosystem.config.js")).unwrap();
        assert_eq!(detect_start_command(&d).as_deref(), Some("npm start"));
        std::fs::write(d.join("package.json"), r#"{"main":"./dist/main.js"}"#).unwrap();
        // main 指向的文件不存在 → 退到入口文件候选
        assert_eq!(detect_start_command(&d).as_deref(), Some("node server.js"));
        std::fs::create_dir_all(d.join("dist")).unwrap();
        std::fs::write(d.join("dist/main.js"), "").unwrap();
        assert_eq!(
            detect_start_command(&d).as_deref(),
            Some("node dist/main.js")
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn unsafe_main_is_ignored_and_empty_project_detects_nothing() {
        let d = project(&[
            ("package.json", r#"{"main":"../../etc/x.js"}"#),
            ("app.js", ""),
        ]);
        assert_eq!(detect_start_command(&d).as_deref(), Some("node app.js"));
        let e = project(&[]);
        assert_eq!(detect_start_command(&e), None);
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_dir_all(&e);
    }

    #[test]
    fn deps_stamp_tracks_package_and_lockfile() {
        let d = project(&[("package.json", r#"{"dependencies":{"a":"1"}}"#)]);
        assert!(deps_outdated(&d), "从未安装");
        std::fs::create_dir_all(d.join("node_modules")).unwrap();
        write_deps_stamp(&d);
        assert!(!deps_outdated(&d), "摘要一致跳过");
        std::fs::write(d.join("package-lock.json"), "{}").unwrap();
        assert!(deps_outdated(&d), "新增锁文件触发重装");
        write_deps_stamp(&d);
        std::fs::write(d.join("package.json"), r#"{"dependencies":{"a":"2"}}"#).unwrap();
        assert!(deps_outdated(&d), "依赖变化触发重装");
        let none = project(&[]);
        assert!(!deps_outdated(&none), "无 package.json 不装");
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_dir_all(&none);
    }

    #[test]
    fn install_and_build_commands() {
        let bin = "/opt/flamepanel/runtimes/node/22/bin";
        let d = project(&[("package.json", r#"{"scripts":{"build":"vite build"}}"#)]);
        let c = install_command(Some(bin), &d);
        assert_eq!(c.program, format!("{bin}/npm"));
        assert_eq!(c.args[0], "install");
        assert_eq!(c.cwd.as_deref(), Some(d.to_string_lossy().as_ref()));
        assert!(c.env.iter().any(|(k, v)| k == "PATH" && v.starts_with(bin)));
        std::fs::write(d.join("package-lock.json"), "{}").unwrap();
        assert_eq!(install_command(None, &d).args[0], "ci");
        assert_eq!(install_command(None, &d).program, "npm");
        let b = build_command(Some(bin), &d).unwrap();
        assert_eq!(b.args, vec!["run", "build"]);
        std::fs::write(d.join("package.json"), "{}").unwrap();
        assert!(build_command(Some(bin), &d).is_none());
        let _ = std::fs::remove_dir_all(&d);
    }
}
