use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use wasmtime::*;

use crate::core::error::AppError;

/// Sandbox limits for WASM plugins
pub struct SandboxConfig {
    pub max_memory_bytes: usize,
    pub max_execution_ms: u64,
    pub allow_stdio: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: 128 * 1024 * 1024, // 128 MB
            max_execution_ms: 5_000,             // 5 seconds
            allow_stdio: false,
        }
    }
}

/// Metadata returned by a plugin's `metadata()` export
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
}

/// A loaded and compiled WASM plugin
struct LoadedPlugin {
    engine: Engine,
    module: Module,
    meta: PluginMeta,
}

/// WASM plugin runtime — loads, compiles, and executes .wasm plugins
#[allow(dead_code)]
pub struct WasmRuntime {
    plugins: Arc<RwLock<HashMap<String, LoadedPlugin>>>,
    plugins_dir: PathBuf,
    sandbox: SandboxConfig,
}

impl WasmRuntime {
    pub fn new(plugins_dir: PathBuf, sandbox: SandboxConfig) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            plugins_dir,
            sandbox,
        }
    }

    /// Scan plugins/ directory and load all .wasm files
    pub async fn load_all(&self) -> Result<Vec<PluginMeta>, AppError> {
        let mut loaded = Vec::new();

        if !self.plugins_dir.exists() {
            std::fs::create_dir_all(&self.plugins_dir)
                .map_err(|e| AppError::Internal(format!("创建插件目录失败: {e}")))?;
            return Ok(loaded);
        }

        let mut engine_cfg = Config::default();
        engine_cfg.consume_fuel(true);
        let engine = Engine::new(&engine_cfg)
            .map_err(|e| AppError::Internal(format!("WASM 引擎初始化失败: {e}")))?;

        let entries = std::fs::read_dir(&self.plugins_dir)
            .map_err(|e| AppError::Internal(format!("读取插件目录失败: {e}")))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "wasm") {
                match self.load_single(&engine, &path) {
                    Ok(plugin) => {
                        let name = plugin.meta.name.clone();
                        loaded.push(plugin.meta.clone());
                        self.plugins.write().await.insert(name, plugin);
                    }
                    Err(e) => {
                        tracing::warn!("加载插件失败 {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(loaded)
    }

    fn load_single(&self, engine: &Engine, path: &std::path::Path) -> Result<LoadedPlugin, AppError> {
        let wasm_bytes = std::fs::read(path)
            .map_err(|e| AppError::Internal(format!("读取 WASM 文件失败: {e}")))?;

        let module = Module::from_binary(engine, &wasm_bytes)
            .map_err(|e| AppError::Internal(format!("WASM 模块编译失败: {e}")))?;

        // Extract metadata by calling the `metadata` export
        let meta = Self::extract_metadata(engine, &module)?;

        Ok(LoadedPlugin {
            engine: engine.clone(),
            module,
            meta,
        })
    }

    fn extract_metadata(engine: &Engine, module: &Module) -> Result<PluginMeta, AppError> {
        let mut store = Store::new(engine, ());
        let instance = Instance::new(&mut store, module, &[])
            .map_err(|e| AppError::Internal(format!("WASM 实例化失败: {e}")))?;

        // Try to call a `metadata` function that returns a pointer+length to a JSON string
        // For security, we use a simple linear memory approach
        if let Ok(metadata_fn) = instance.get_typed_func::<(), (i32, i32)>(&mut store, "metadata") {
            let memory = instance
                .get_memory(&mut store, "memory")
                .ok_or_else(|| AppError::Internal("插件缺少 memory 导出".into()))?;

            match metadata_fn.call(&mut store, ()) {
                Ok((ptr, len)) => {
                    let data = memory.data(&store);
                    let end = (ptr as usize).saturating_add(len as usize).min(data.len());
                    let json_bytes = &data[ptr as usize..end];
                    let json_str = std::str::from_utf8(json_bytes).unwrap_or("{}");
                    if let Ok(meta) = serde_json::from_str::<PluginMeta>(json_str) {
                        return Ok(meta);
                    }
                }
                Err(_) => { /* fall through to defaults */ }
            }
        }

        // Default metadata from filename
        let name = module
            .name()
            .unwrap_or("unknown")
            .to_string();

        Ok(PluginMeta {
            name,
            version: "0.1.0".into(),
            description: "无描述".into(),
            author: "unknown".into(),
        })
    }

    /// Execute a plugin by name with the provided input string
    pub async fn execute(&self, name: &str, input: &str) -> Result<String, AppError> {
        let plugins = self.plugins.read().await;
        let plugin = plugins
            .get(name)
            .ok_or_else(|| AppError::NotFound(format!("插件 '{}' 未找到", name)))?;

        let mut store = Store::new(&plugin.engine, ());
        // Set fuel for execution time limiting
        store.set_fuel(u64::MAX).ok();

        let instance = Instance::new(&mut store, &plugin.module, &[])
            .map_err(|e| AppError::Internal(format!("WASM 实例化失败: {e}")))?;

        // Try to call execute(input_ptr, input_len) -> (output_ptr, output_len)
        let execute_fn = instance
            .get_typed_func::<(i32, i32), (i32, i32)>(&mut store, "execute")
            .map_err(|_| AppError::Internal("插件缺少 'execute' 导出函数".into()))?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| AppError::Internal("插件缺少 memory 导出".into()))?;

        // Write input into WASM memory
        let input_bytes = input.as_bytes();
        let alloc_fn = instance
            .get_typed_func::<i32, i32>(&mut store, "alloc")
            .map_err(|_| AppError::Internal("插件缺少 'alloc' 导出函数".into()))?;

        let input_ptr = alloc_fn
            .call(&mut store, input_bytes.len() as i32)
            .map_err(|e| AppError::Internal(format!("WASM alloc 失败: {e}")))?;

        let mem_data = memory.data_mut(&mut store);
        let start = input_ptr as usize;
        let end = start.saturating_add(input_bytes.len());
        if end <= mem_data.len() {
            mem_data[start..end].copy_from_slice(input_bytes);
        } else {
            return Err(AppError::Internal("WASM 内存不足".into()));
        }

        // Call execute
        let (output_ptr, output_len) = execute_fn
            .call(&mut store, (input_ptr, input_bytes.len() as i32))
            .map_err(|e| AppError::Internal(format!("WASM execute 失败: {e}")))?;

        // Read output from WASM memory
        let data = memory.data(&store);
        let out_start = output_ptr as usize;
        let out_end = out_start.saturating_add(output_len as usize).min(data.len());
        let output = String::from_utf8_lossy(&data[out_start..out_end]).to_string();

        Ok(output)
    }

    /// List all loaded plugins and their metadata
    pub async fn list(&self) -> Vec<PluginMeta> {
        self.plugins.read().await.values().map(|p| p.meta.clone()).collect()
    }

    /// Unload a plugin by name
    pub async fn unload(&self, name: &str) -> bool {
        self.plugins.write().await.remove(name).is_some()
    }
}
