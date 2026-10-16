//! 项目启动命令展开（S-11 · Doc/20 §5.3）。
//!
//! 守护单元（systemd `ExecStart=`）不经 shell：程序名必须是绝对路径，且命令里禁止
//! `$` 等 shell 元字符（`SystemdSupervisor::parse_exec`）。于是用户写的 `node server.js`
//! 会被解析成 `<deploy_path>/node` 然后因路径前缀校验被拒——Node 项目在 S-11 之前
//! 实际上无法用最自然的写法启动。
//!
//! 本模块在交给守护器**之前**做纯文本展开（存库的仍是用户原文）：
//! - `$RUNTIME` / `${RUNTIME}` → 绑定运行时的可执行目录（如 `/opt/flamepanel/runtimes/node/22/bin`）；
//! - `$DEPLOY` / `${DEPLOY}` → 项目部署目录；
//! - 首个词是运行时自带的裸命令（Node：`node` / `npm` / `npx` / `corepack` / `pm2` / `pm2-runtime`）
//!   时补全为 `<bin>/<cmd>`。
//!
//! 引用了 `$RUNTIME` 却没有绑定运行时属于配置错误，返回可读的错误而不是让守护器报「非法字符」。

/// 展开上下文。
#[derive(Debug, Clone, Copy)]
pub struct CommandContext<'a> {
    /// 绑定运行时的可执行目录（未绑定为 None）
    pub runtime_bin: Option<&'a str>,
    /// 需要补全到 `runtime_bin` 的裸命令白名单（按运行时种类给出）
    pub runtime_binaries: &'a [&'a str],
    /// 项目部署目录
    pub deploy_path: &'a str,
}

/// 展开启动命令；`Err` 为面向用户的中文原因。
pub fn expand(command: &str, ctx: &CommandContext<'_>) -> Result<String, String> {
    let deploy = ctx.deploy_path.trim_end_matches('/');
    let mut out = replace_var(command.trim(), "DEPLOY", deploy);
    if mentions_var(&out, "RUNTIME") {
        let bin = ctx
            .runtime_bin
            .ok_or_else(|| "启动命令引用了 $RUNTIME，但项目未绑定运行时".to_string())?;
        out = replace_var(&out, "RUNTIME", bin.trim_end_matches('/'));
    }
    if let Some(bin) = ctx.runtime_bin {
        let (first, rest) = split_first(&out);
        if ctx.runtime_binaries.contains(&first) {
            out = format!("{}/{}{}", bin.trim_end_matches('/'), first, rest);
        }
    }
    Ok(out)
}

fn mentions_var(s: &str, name: &str) -> bool {
    s.contains(&format!("${{{name}}}")) || find_bare_var(s, name).is_some()
}

/// 替换 `${NAME}` 与 `$NAME`（后者要求变量名后不是标识符字符，避免 `$RUNTIMEX` 误伤）。
fn replace_var(s: &str, name: &str, value: &str) -> String {
    let mut out = s.replace(&format!("${{{name}}}"), value);
    while let Some(pos) = find_bare_var(&out, name) {
        out.replace_range(pos..pos + name.len() + 1, value);
    }
    out
}

fn find_bare_var(s: &str, name: &str) -> Option<usize> {
    let pat = format!("${name}");
    let mut from = 0;
    while let Some(i) = s[from..].find(&pat) {
        let pos = from + i;
        let end = pos + pat.len();
        let boundary = s[end..]
            .chars()
            .next()
            .is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'));
        if boundary {
            return Some(pos);
        }
        from = end;
    }
    None
}

fn split_first(s: &str) -> (&str, &str) {
    match s.find(char::is_whitespace) {
        Some(i) => (&s[..i], &s[i..]),
        None => (s, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NODE: &[&str] = &["node", "npm", "npx", "corepack", "pm2", "pm2-runtime"];
    const BIN: &str = "/opt/flamepanel/runtimes/node/22/bin";

    fn ctx(bin: Option<&'static str>) -> CommandContext<'static> {
        CommandContext {
            runtime_bin: bin,
            runtime_binaries: NODE,
            deploy_path: "/opt/flamepanel/projects/blog/",
        }
    }

    #[test]
    fn bare_runtime_commands_get_bin_prefix() {
        let c = ctx(Some(BIN));
        assert_eq!(
            expand("node server.js", &c).unwrap(),
            format!("{BIN}/node server.js")
        );
        assert_eq!(
            expand("  npm start ", &c).unwrap(),
            format!("{BIN}/npm start")
        );
        assert_eq!(
            expand("pm2-runtime start ecosystem.config.js", &c).unwrap(),
            format!("{BIN}/pm2-runtime start ecosystem.config.js")
        );
        assert_eq!(expand("npx", &c).unwrap(), format!("{BIN}/npx"));
    }

    #[test]
    fn non_runtime_and_absolute_programs_untouched() {
        let c = ctx(Some(BIN));
        assert_eq!(
            expand("/usr/bin/node a.js", &c).unwrap(),
            "/usr/bin/node a.js"
        );
        assert_eq!(expand("./start.sh", &c).unwrap(), "./start.sh");
        // 仅首词：参数里的 node 不补全
        assert_eq!(
            expand("./run node", &c).unwrap(),
            "./run node",
            "只看程序名"
        );
        assert_eq!(expand("nodemon app.js", &c).unwrap(), "nodemon app.js");
    }

    #[test]
    fn variables_expand_in_both_forms() {
        let c = ctx(Some(BIN));
        assert_eq!(
            expand("$RUNTIME/node $DEPLOY/dist/main.js", &c).unwrap(),
            format!("{BIN}/node /opt/flamepanel/projects/blog/dist/main.js")
        );
        assert_eq!(
            expand("${RUNTIME}/npm run start --prefix ${DEPLOY}", &c).unwrap(),
            format!("{BIN}/npm run start --prefix /opt/flamepanel/projects/blog")
        );
        // 标识符边界：$RUNTIMEX 不是 $RUNTIME
        assert_eq!(expand("./x $RUNTIMEX", &c).unwrap(), "./x $RUNTIMEX");
    }

    #[test]
    fn runtime_variable_without_binding_is_a_readable_error() {
        let c = ctx(None);
        let err = expand("$RUNTIME/node a.js", &c).unwrap_err();
        assert!(err.contains("未绑定运行时"), "{err}");
        // 未绑定时裸命令保持原样（交给守护器按既有规则处理）
        assert_eq!(expand("node a.js", &c).unwrap(), "node a.js");
        // $DEPLOY 不依赖运行时
        assert_eq!(
            expand("$DEPLOY/bin/app", &c).unwrap(),
            "/opt/flamepanel/projects/blog/bin/app"
        );
    }
}
