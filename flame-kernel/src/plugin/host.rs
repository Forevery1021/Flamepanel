//! WASM 插件宿主 API（导入模块 `flame`，Doc/21 §5.7）。
//!
//! ## ABI
//!
//! 所有字符串 / 字节均以 `(ptr, len)` 指向插件**导出的 `memory`** 中的区间（越界即 `ERR_INVALID`）。
//! 需要向插件返回数据的函数采用「调用方给缓冲区」约定：`(…, out_ptr, out_cap) -> i32`——
//! 返回值 ≥ 0 为**完整数据长度**，宿主写入 `min(长度, out_cap)` 字节；返回值 > `out_cap`
//! 表示缓冲区不足，插件可按返回值扩容后重调。负数为错误码：
//!
//! | 码 | 含义 |
//! |----|------|
//! | -1 `ERR_DENIED` | 未授予所需能力 / 路径不在能力根内 |
//! | -2 `ERR_INVALID` | 参数非法（越界、非 UTF-8、对目录读文件…） |
//! | -3 `ERR_NOT_FOUND` | 文件 / 设置不存在 |
//! | -4 `ERR_IO` | 宿主 IO 失败 |
//! | -5 `ERR_TOO_LARGE` | 超过单次上限（读 / 写 1 MiB、输出 1 MiB、设置值 4 KiB） |
//! | -6 `ERR_UNAVAILABLE` | 宿主暂无数据（如指标尚未采样） |
//!
//! ## 函数
//!
//! | 函数 | 能力 | 说明 |
//! |------|------|------|
//! | `log(level, ptr, len)` | — | 0 trace … 4 error；每次执行最多 100 条，每条截断到 1 KiB |
//! | `input_len() -> i32` / `input_read(out, cap) -> i32` | — | 读取本次调用的输入（API `input`） |
//! | `output(ptr, len) -> i32` | — | 设置本次调用的输出（覆盖式；未调用时沿用旧语义：返回值字节） |
//! | `setting_get(kp, kl, out, cap) -> i32` | — | 读自身设置 |
//! | `setting_set(kp, kl, vp, vl) -> i32` | `settings:write` | 写自身设置（执行结束后持久化） |
//! | `fs_read(pp, pl, out, cap) -> i32` | `fs:read:<dir>` | 读文件 |
//! | `fs_list(pp, pl, out, cap) -> i32` | `fs:read:<dir>` | 列目录，JSON `[{name,is_dir,size}]`（最多 1000 项） |
//! | `fs_write(pp, pl, dp, dl) -> i32` | `fs:write:<dir>` | 覆盖写文件，返回 0 |
//! | `metrics(out, cap) -> i32` | `metrics:read` | 面板本机最新指标快照 JSON |

#[cfg(feature = "wasm")]
use super::capability::AccessError;
use super::capability::Capabilities;
use std::collections::HashMap;
use std::sync::Arc;

pub const ERR_DENIED: i32 = -1;
pub const ERR_INVALID: i32 = -2;
pub const ERR_NOT_FOUND: i32 = -3;
pub const ERR_IO: i32 = -4;
pub const ERR_TOO_LARGE: i32 = -5;
pub const ERR_UNAVAILABLE: i32 = -6;

/// 单次读 / 写文件、输出、输入的字节上限
pub const MAX_IO_BYTES: usize = 1024 * 1024;
pub const MAX_LOG_LINES: usize = 100;

/// 宿主侧数据来源（由内核启动时注入，避免插件模块依赖具体基础设施）。
pub trait HostContext: Send + Sync {
    /// 面板本机最新指标快照（JSON）；尚未采样返回 None
    fn metrics_json(&self) -> Option<String>;
}

/// 一次插件调用的上下文（输入 + 授予能力 + 设置快照）。
#[derive(Clone, Default)]
pub struct Invocation {
    pub plugin_id: String,
    pub capabilities: Capabilities,
    pub settings: HashMap<String, String>,
    pub input: Vec<u8>,
    pub ctx: Option<Arc<dyn HostContext>>,
}

/// 调用产物：显式输出、设置变更、日志。
#[derive(Debug, Default, Clone)]
pub struct HostOutcome {
    pub output: Option<Vec<u8>>,
    /// 执行期间设置被修改时为修改后的完整设置
    pub settings: Option<HashMap<String, String>>,
    pub logs: Vec<String>,
}

#[cfg(feature = "wasm")]
pub(crate) use wasm_impl::{add_to_linker, HostState};

#[cfg(feature = "wasm")]
mod wasm_impl {
    use super::*;
    use wasmtime::{Caller, Extern, Linker};

    const MAX_LOG_LINE: usize = 1024;
    const MAX_LIST_ENTRIES: usize = 1000;
    const MAX_SETTING_VALUE: usize = 4096;
    const MAX_SETTINGS: usize = 256;

    pub(crate) struct HostState {
        pub(crate) limiter: crate::plugin::sandbox::MemoryLimiter,
        inv: Invocation,
        settings_dirty: bool,
        outcome: HostOutcome,
    }

    impl HostState {
        pub(crate) fn new(limiter: crate::plugin::sandbox::MemoryLimiter, inv: Invocation) -> Self {
            Self {
                limiter,
                inv,
                settings_dirty: false,
                outcome: HostOutcome::default(),
            }
        }

        pub(crate) fn into_outcome(mut self) -> HostOutcome {
            if self.settings_dirty {
                self.outcome.settings = Some(self.inv.settings);
            }
            self.outcome
        }
    }

    fn access_code(e: AccessError) -> i32 {
        match e {
            AccessError::Denied => ERR_DENIED,
            AccessError::Invalid => ERR_INVALID,
            AccessError::NotFound => ERR_NOT_FOUND,
        }
    }

    fn range(ptr: i32, len: i32) -> Option<std::ops::Range<usize>> {
        let (p, l) = (usize::try_from(ptr).ok()?, usize::try_from(len).ok()?);
        Some(p..p.checked_add(l)?)
    }

    /// 从插件内存拷出 `[ptr, ptr+len)`。
    fn read_mem(caller: &mut Caller<'_, HostState>, ptr: i32, len: i32) -> Result<Vec<u8>, i32> {
        let Some(Extern::Memory(mem)) = caller.get_export("memory") else {
            return Err(ERR_INVALID);
        };
        let r = range(ptr, len).ok_or(ERR_INVALID)?;
        mem.data(&caller)
            .get(r)
            .map(<[u8]>::to_vec)
            .ok_or(ERR_INVALID)
    }

    fn read_str(caller: &mut Caller<'_, HostState>, ptr: i32, len: i32) -> Result<String, i32> {
        String::from_utf8(read_mem(caller, ptr, len)?).map_err(|_| ERR_INVALID)
    }

    /// 「调用方给缓冲区」约定：写入 `min(len, cap)`，返回完整长度。
    fn write_out(caller: &mut Caller<'_, HostState>, data: &[u8], out: i32, cap: i32) -> i32 {
        let Ok(total) = i32::try_from(data.len()) else {
            return ERR_TOO_LARGE;
        };
        let Some(Extern::Memory(mem)) = caller.get_export("memory") else {
            return ERR_INVALID;
        };
        let n = data.len().min(usize::try_from(cap).unwrap_or(0));
        let Some(r) = range(out, n as i32) else {
            return ERR_INVALID;
        };
        match mem.data_mut(caller).get_mut(r) {
            Some(dst) => {
                dst.copy_from_slice(&data[..n]);
                total
            }
            None => ERR_INVALID,
        }
    }

    macro_rules! tri {
        ($e:expr) => {
            match $e {
                Ok(v) => v,
                Err(code) => return code,
            }
        };
    }

    /// 把 `flame.*` 宿主函数注册进 linker。未列出的任何导入在实例化时失败（deny-by-default）。
    pub(crate) fn add_to_linker(linker: &mut Linker<HostState>) -> wasmtime::Result<()> {
        linker.func_wrap(
            "flame",
            "log",
            |mut caller: Caller<'_, HostState>, level: i32, ptr: i32, len: i32| {
                if caller.data().outcome.logs.len() >= MAX_LOG_LINES {
                    return;
                }
                let Ok(bytes) = read_mem(&mut caller, ptr, len.min(MAX_LOG_LINE as i32)) else {
                    return;
                };
                let msg = String::from_utf8_lossy(&bytes).to_string();
                let id = caller.data().inv.plugin_id.clone();
                match level {
                    i32::MIN..=0 => tracing::trace!(target: "plugin", plugin = %id, "{msg}"),
                    1 => tracing::debug!(target: "plugin", plugin = %id, "{msg}"),
                    2 => tracing::info!(target: "plugin", plugin = %id, "{msg}"),
                    3 => tracing::warn!(target: "plugin", plugin = %id, "{msg}"),
                    _ => tracing::error!(target: "plugin", plugin = %id, "{msg}"),
                }
                let tag = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"][level.clamp(0, 4) as usize];
                caller
                    .data_mut()
                    .outcome
                    .logs
                    .push(format!("[{tag}] {msg}"));
            },
        )?;

        linker.func_wrap("flame", "input_len", |caller: Caller<'_, HostState>| {
            caller.data().inv.input.len() as i32
        })?;

        linker.func_wrap(
            "flame",
            "input_read",
            |mut caller: Caller<'_, HostState>, out: i32, cap: i32| {
                let input = caller.data().inv.input.clone();
                write_out(&mut caller, &input, out, cap)
            },
        )?;

        linker.func_wrap(
            "flame",
            "output",
            |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| {
                if usize::try_from(len).unwrap_or(usize::MAX) > MAX_IO_BYTES {
                    return ERR_TOO_LARGE;
                }
                let data = tri!(read_mem(&mut caller, ptr, len));
                caller.data_mut().outcome.output = Some(data);
                0
            },
        )?;

        linker.func_wrap(
            "flame",
            "setting_get",
            |mut caller: Caller<'_, HostState>, kp: i32, kl: i32, out: i32, cap: i32| {
                let key = tri!(read_str(&mut caller, kp, kl));
                let Some(v) = caller.data().inv.settings.get(&key).cloned() else {
                    return ERR_NOT_FOUND;
                };
                write_out(&mut caller, v.as_bytes(), out, cap)
            },
        )?;

        linker.func_wrap(
            "flame",
            "setting_set",
            |mut caller: Caller<'_, HostState>, kp: i32, kl: i32, vp: i32, vl: i32| {
                if !caller.data().inv.capabilities.settings_write() {
                    return ERR_DENIED;
                }
                if usize::try_from(vl).unwrap_or(usize::MAX) > MAX_SETTING_VALUE {
                    return ERR_TOO_LARGE;
                }
                let key = tri!(read_str(&mut caller, kp, kl));
                let value = tri!(read_str(&mut caller, vp, vl));
                if !valid_setting_key(&key) {
                    return ERR_INVALID;
                }
                let st = caller.data_mut();
                if !st.inv.settings.contains_key(&key) && st.inv.settings.len() >= MAX_SETTINGS {
                    return ERR_TOO_LARGE;
                }
                st.inv.settings.insert(key, value);
                st.settings_dirty = true;
                0
            },
        )?;

        linker.func_wrap(
            "flame",
            "fs_read",
            |mut caller: Caller<'_, HostState>, pp: i32, pl: i32, out: i32, cap: i32| {
                let path = tri!(read_str(&mut caller, pp, pl));
                let target = tri!(caller
                    .data()
                    .inv
                    .capabilities
                    .resolve_read(&path)
                    .map_err(access_code));
                let meta = tri!(std::fs::metadata(&target).map_err(|_| ERR_IO));
                if !meta.is_file() {
                    return ERR_INVALID;
                }
                if meta.len() > MAX_IO_BYTES as u64 {
                    return ERR_TOO_LARGE;
                }
                let data = tri!(std::fs::read(&target).map_err(|_| ERR_IO));
                write_out(&mut caller, &data, out, cap)
            },
        )?;

        linker.func_wrap(
            "flame",
            "fs_list",
            |mut caller: Caller<'_, HostState>, pp: i32, pl: i32, out: i32, cap: i32| {
                let path = tri!(read_str(&mut caller, pp, pl));
                let target = tri!(caller
                    .data()
                    .inv
                    .capabilities
                    .resolve_read(&path)
                    .map_err(access_code));
                let rd = tri!(std::fs::read_dir(&target).map_err(|_| ERR_INVALID));
                let mut entries: Vec<serde_json::Value> = rd
                    .filter_map(Result::ok)
                    .take(MAX_LIST_ENTRIES)
                    .map(|e| {
                        let meta = e.metadata().ok();
                        serde_json::json!({
                            "name": e.file_name().to_string_lossy(),
                            "is_dir": meta.as_ref().is_some_and(|m| m.is_dir()),
                            "size": meta.map(|m| m.len()).unwrap_or(0),
                        })
                    })
                    .collect();
                entries.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
                let json = serde_json::Value::Array(entries).to_string();
                write_out(&mut caller, json.as_bytes(), out, cap)
            },
        )?;

        linker.func_wrap(
            "flame",
            "fs_write",
            |mut caller: Caller<'_, HostState>, pp: i32, pl: i32, dp: i32, dl: i32| {
                if usize::try_from(dl).unwrap_or(usize::MAX) > MAX_IO_BYTES {
                    return ERR_TOO_LARGE;
                }
                let path = tri!(read_str(&mut caller, pp, pl));
                let data = tri!(read_mem(&mut caller, dp, dl));
                let target = tri!(caller
                    .data()
                    .inv
                    .capabilities
                    .resolve_write(&path)
                    .map_err(access_code));
                tri!(std::fs::write(&target, &data).map_err(|_| ERR_IO));
                0
            },
        )?;

        linker.func_wrap(
            "flame",
            "metrics",
            |mut caller: Caller<'_, HostState>, out: i32, cap: i32| {
                if !caller.data().inv.capabilities.metrics_read() {
                    return ERR_DENIED;
                }
                let Some(json) = caller
                    .data()
                    .inv
                    .ctx
                    .as_ref()
                    .and_then(|c| c.metrics_json())
                else {
                    return ERR_UNAVAILABLE;
                };
                write_out(&mut caller, json.as_bytes(), out, cap)
            },
        )?;

        Ok(())
    }
}

/// 设置键：1–128 位 `[A-Za-z0-9._-]`（与 `/api/plugins/{id}/settings` 一致）
pub fn valid_setting_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 128
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

#[cfg(all(test, feature = "wasm"))]
mod tests {
    use super::*;
    use crate::plugin::sandbox::{PluginConfig, WasmSandbox};

    fn inv(caps: &[&str], input: &str) -> Invocation {
        Invocation {
            plugin_id: "t".into(),
            capabilities: Capabilities::parse_all(
                &caps.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            )
            .unwrap(),
            input: input.as_bytes().to_vec(),
            ..Default::default()
        }
    }

    fn run(
        wat_src: &str,
        inv: Invocation,
    ) -> Result<(Vec<u8>, HostOutcome), crate::core::error::AppError> {
        let wasm = wat::parse_str(wat_src).unwrap();
        WasmSandbox::new(&PluginConfig::default())
            .unwrap()
            .execute_with(&wasm, "run", 10_000_000, inv)
    }

    fn ret(bytes: &[u8]) -> i32 {
        i32::from_le_bytes(bytes[..4].try_into().unwrap())
    }

    /// 通用探针：把 data 段 [0,plen) 当路径调用 `$f(path, out=4096, cap=8192)`，输出读到的内容并返回码
    fn fs_probe(func: &str, path: &str) -> String {
        format!(
            r#"(module
  (import "flame" "{func}" (func $f (param i32 i32 i32 i32) (result i32)))
  (import "flame" "output" (func $output (param i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "{path}")
  (func (export "run") (result i32)
    (local $r i32)
    (local.set $r (call $f (i32.const 0) (i32.const {len}) (i32.const 4096) (i32.const 8192)))
    (if (i32.gt_s (local.get $r) (i32.const 0))
      (then (drop (call $output (i32.const 4096) (local.get $r)))))
    (local.get $r)))"#,
            len = path.len()
        )
    }

    #[test]
    fn hello_builtin_reads_input_writes_output_and_logs() {
        let (_, out) = run(super::super::market::HELLO_WAT, inv(&[], "Bob")).unwrap();
        assert_eq!(out.output.as_deref(), Some(&b"Hello, Bob!"[..]));
        assert_eq!(out.logs, vec!["[INFO] hello plugin invoked"]);
        assert!(out.settings.is_none());
        let (_, out) = run(super::super::market::HELLO_WAT, inv(&[], "")).unwrap();
        assert_eq!(out.output.as_deref(), Some(&b"Hello, FlamePanel!"[..]));
    }

    #[test]
    fn fs_read_requires_capability_and_stays_in_root() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.log"), "line1\n").unwrap();
        let file = dir.path().join("a.log");
        let f = file.to_str().unwrap();
        let root = format!("fs:read:{}", dir.path().display());

        let (r, out) = run(&fs_probe("fs_read", f), inv(&[], "")).unwrap();
        assert_eq!(ret(&r), ERR_DENIED, "无能力");
        assert!(out.output.is_none());

        let (r, out) = run(&fs_probe("fs_read", f), inv(&[&root], "")).unwrap();
        assert_eq!(ret(&r), 6);
        assert_eq!(out.output.as_deref(), Some(&b"line1\n"[..]));

        let (r, _) = run(&fs_probe("fs_read", "/etc/hostname"), inv(&[&root], "")).unwrap();
        assert!(ret(&r) == ERR_DENIED || ret(&r) == ERR_NOT_FOUND, "根外");
        let (r, _) = run(&fs_probe("fs_read", "missing.log"), inv(&[&root], "")).unwrap();
        assert_eq!(ret(&r), ERR_NOT_FOUND);

        let (r, out) = run(&fs_probe("fs_list", ""), inv(&[&root], "")).unwrap();
        // 空路径非法
        assert_eq!(ret(&r), ERR_INVALID);
        assert!(out.output.is_none());
        let d = dir.path().to_str().unwrap();
        let (_, out) = run(&fs_probe("fs_list", d), inv(&[&root], "")).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&out.output.unwrap()).unwrap();
        assert_eq!(v[0]["name"], "a.log");
        assert_eq!(v[0]["size"], 6);
    }

    #[test]
    fn fs_write_requires_write_capability() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("out.txt");
        let t = target.to_str().unwrap();
        let wat_src = format!(
            r#"(module
  (import "flame" "fs_write" (func $w (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "{t}")
  (data (i32.const 2048) "payload")
  (func (export "run") (result i32)
    (call $w (i32.const 0) (i32.const {len}) (i32.const 2048) (i32.const 7))))"#,
            len = t.len()
        );
        let read_only = format!("fs:read:{}", dir.path().display());
        let (r, _) = run(&wat_src, inv(&[&read_only], "")).unwrap();
        assert_eq!(ret(&r), ERR_DENIED, "读能力不含写");
        assert!(!target.exists());
        let rw = format!("fs:write:{}", dir.path().display());
        let (r, _) = run(&wat_src, inv(&[&rw], "")).unwrap();
        assert_eq!(ret(&r), 0);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "payload");
    }

    struct FakeCtx;
    impl HostContext for FakeCtx {
        fn metrics_json(&self) -> Option<String> {
            Some(r#"{"cpu":1.5}"#.into())
        }
    }

    #[test]
    fn metrics_requires_capability_and_context() {
        let src = super::super::market::METRICS_WAT;
        let (r, out) = run(src, inv(&[], "")).unwrap();
        assert_eq!(ret(&r), ERR_DENIED);
        assert_eq!(
            out.output.as_deref(),
            Some(&br#"{"error":"metrics unavailable"}"#[..])
        );
        let (r, _) = run(src, inv(&["metrics:read"], "")).unwrap();
        assert_eq!(ret(&r), ERR_UNAVAILABLE, "未注入上下文");
        let mut i = inv(&["metrics:read"], "");
        i.ctx = Some(Arc::new(FakeCtx));
        let (r, out) = run(src, i).unwrap();
        assert_eq!(ret(&r), 0);
        assert_eq!(out.output.as_deref(), Some(&br#"{"cpu":1.5}"#[..]));
    }

    #[test]
    fn settings_get_and_set() {
        let src = r#"(module
  (import "flame" "setting_get" (func $get (param i32 i32 i32 i32) (result i32)))
  (import "flame" "setting_set" (func $set (param i32 i32 i32 i32) (result i32)))
  (import "flame" "output" (func $output (param i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "greeting")
  (data (i32.const 16) "count")
  (data (i32.const 32) "42")
  (func (export "run") (result i32)
    (local $n i32)
    (local.set $n (call $get (i32.const 0) (i32.const 8) (i32.const 1024) (i32.const 64)))
    (if (i32.gt_s (local.get $n) (i32.const 0))
      (then (drop (call $output (i32.const 1024) (local.get $n)))))
    (call $set (i32.const 16) (i32.const 5) (i32.const 32) (i32.const 2))))"#;
        let mut i = inv(&[], "");
        i.settings.insert("greeting".into(), "hi".into());
        let (r, out) = run(src, i.clone()).unwrap();
        assert_eq!(ret(&r), ERR_DENIED, "读允许、写需 settings:write");
        assert_eq!(out.output.as_deref(), Some(&b"hi"[..]));
        assert!(out.settings.is_none());

        i.capabilities = Capabilities::parse_all(&["settings:write".into()]).unwrap();
        let (r, out) = run(src, i).unwrap();
        assert_eq!(ret(&r), 0);
        let s = out.settings.unwrap();
        assert_eq!(s.get("count").map(String::as_str), Some("42"));
        assert_eq!(s.get("greeting").map(String::as_str), Some("hi"));
    }

    #[test]
    fn out_of_bounds_pointers_are_rejected() {
        let src = r#"(module
  (import "flame" "output" (func $output (param i32 i32) (result i32)))
  (memory (export "memory") 1)
  (func (export "run") (result i32)
    (call $output (i32.const 65530) (i32.const 100))))"#;
        let (r, out) = run(src, inv(&[], "")).unwrap();
        assert_eq!(ret(&r), ERR_INVALID);
        assert!(out.output.is_none());
    }

    #[test]
    fn unknown_imports_fail_instantiation() {
        let src = r#"(module
  (import "flame" "exec" (func $exec (param i32 i32) (result i32)))
  (func (export "run") (result i32) (i32.const 0)))"#;
        let err = run(src, inv(&[], "")).unwrap_err().to_string();
        assert!(err.contains("instantiate"), "{err}");
    }

    #[test]
    fn wall_clock_timeout_interrupts_infinite_loop() {
        let wasm = wat::parse_str(r#"(module (func (export "run") (loop $l (br $l))))"#).unwrap();
        let cfg = PluginConfig {
            timeout_ms: 200,
            ..PluginConfig::default()
        };
        let start = std::time::Instant::now();
        let err = WasmSandbox::new(&cfg)
            .unwrap()
            .execute_with(&wasm, "run", u64::MAX / 4, Invocation::default())
            .unwrap_err()
            .to_string();
        assert!(err.contains("timed out after 200 ms"), "{err}");
        assert!(start.elapsed() < std::time::Duration::from_secs(10));
    }

    #[test]
    fn fuel_exhaustion_has_stable_error() {
        let wasm = wat::parse_str(r#"(module (func (export "run") (loop $l (br $l))))"#).unwrap();
        let err = WasmSandbox::new(&PluginConfig::default())
            .unwrap()
            .execute_with(&wasm, "run", 10_000, Invocation::default())
            .unwrap_err()
            .to_string();
        assert!(err.contains("fuel"), "{err}");
    }

    #[test]
    fn log_lines_are_capped() {
        let src = r#"(module
  (import "flame" "log" (func $log (param i32 i32 i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "x")
  (func (export "run")
    (local $i i32)
    (loop $l
      (call $log (i32.const 9) (i32.const 0) (i32.const 1))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $l (i32.lt_u (local.get $i) (i32.const 500))))))"#;
        let (_, out) = run(src, inv(&[], "")).unwrap();
        assert_eq!(out.logs.len(), MAX_LOG_LINES);
        assert_eq!(out.logs[0], "[ERROR] x");
    }

    #[test]
    fn setting_key_validation() {
        assert!(valid_setting_key("a.b_c-1"));
        assert!(!valid_setting_key(""));
        assert!(!valid_setting_key("a/b"));
        assert!(!valid_setting_key(&"k".repeat(129)));
    }
}
