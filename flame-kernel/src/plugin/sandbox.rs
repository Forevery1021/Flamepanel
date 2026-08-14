use crate::core::error::AppError;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct PluginConfig {
    pub memory_limit_bytes: usize,
    pub timeout_ms: u64,
    pub max_stack_size: usize,
    pub settings: HashMap<String, String>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            memory_limit_bytes: 64 * 1024 * 1024,
            timeout_ms: 30_000,
            max_stack_size: 1024 * 1024,
            settings: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginStatus {
    Unloaded,
    Loaded,
    Running,
    Disabled,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct PluginMetrics {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub avg_execution_ms: f64,
    pub max_execution_ms: u64,
    pub min_execution_ms: u64,
    pub last_execution_ms: u64,
    pub peak_memory_bytes: usize,
}

impl Default for PluginMetrics {
    fn default() -> Self {
        Self {
            total_executions: 0,
            successful_executions: 0,
            failed_executions: 0,
            avg_execution_ms: 0.0,
            max_execution_ms: 0,
            min_execution_ms: u64::MAX,
            last_execution_ms: 0,
            peak_memory_bytes: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SandboxedPlugin {
    pub id: String,
    pub status: PluginStatus,
    pub wasm_bytes: Vec<u8>,
    /// WASM 字节码 SHA-256 哈希（用于完整性校验）
    pub wasm_hash: String,
    pub config: PluginConfig,
    pub metrics: PluginMetrics,
    pub loaded_at: DateTime<Utc>,
    pub last_executed_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

/// 校验 WASM 字节码哈希（sha2 实现，不依赖 wasmtime；所有 feature 下可用）。
/// 若提供了期望值且不匹配则拒绝，返回稳定错误。
pub fn verify_wasm_hash(wasm_bytes: &[u8], expected: Option<&str>) -> Result<(), AppError> {
    if let Some(expected) = expected {
        if expected.is_empty() {
            return Ok(());
        }
        let mut hasher = Sha256::new();
        hasher.update(wasm_bytes);
        let actual = format!("{:x}", hasher.finalize());
        if actual != expected {
            return Err(AppError::BadRequest(format!(
                "WASM hash mismatch: expected {}, got {}",
                expected, actual
            )));
        }
    }
    Ok(())
}

/// WASM 执行沙箱（wasmtime 实现，feature `wasm` 开启时可用）。
/// 关闭 `wasm` feature 时：插件仅保留元数据/哈希校验，不执行字节码。
#[cfg(feature = "wasm")]
pub struct WasmSandbox {
    engine: wasmtime::Engine,
    /// 每个插件执行的内存上限（字节）
    memory_limit_bytes: usize,
}

/// 资源限制器：限制每个 WASM 实例的线性内存大小（memory.grow 上限）
#[cfg(feature = "wasm")]
#[derive(Debug, Clone)]
struct MemoryLimiter {
    max_memory_bytes: usize,
}

#[cfg(feature = "wasm")]
impl wasmtime::ResourceLimiter for MemoryLimiter {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        Ok(desired <= self.max_memory_bytes)
    }

    fn table_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        // 表格元素数上限：防止实例表无限增长（每个元素占一个指针宽度）
        Ok(desired <= self.max_memory_bytes / std::mem::size_of::<usize>())
    }
}

#[cfg(feature = "wasm")]
impl WasmSandbox {
    pub fn new(config: &PluginConfig) -> Result<Self, AppError> {
        let mut config_builder = wasmtime::Config::new();
        config_builder.consume_fuel(true);
        // 限制 WASM 栈大小，防止栈溢出耗尽宿主内存
        config_builder.max_wasm_stack(config.max_stack_size);
        let engine = wasmtime::Engine::new(&config_builder)
            .map_err(|e| AppError::internal(format!("Failed to create WASM engine: {}", e)))?;
        Ok(Self {
            engine,
            memory_limit_bytes: config.memory_limit_bytes,
        })
    }

    pub fn execute(
        &self,
        wasm_bytes: &[u8],
        function: &str,
        fuel_limit: u64,
    ) -> Result<Vec<u8>, AppError> {
        let module = wasmtime::Module::new(&self.engine, wasm_bytes)
            .map_err(|e| AppError::internal(format!("Failed to compile WASM module: {}", e)))?;
        // 将资源限制器作为 store 数据，限制每个实例的线性内存/表格大小
        let limiter = MemoryLimiter {
            max_memory_bytes: self.memory_limit_bytes,
        };
        let mut store = wasmtime::Store::new(&self.engine, limiter);
        store
            .set_fuel(fuel_limit)
            .map_err(|e| AppError::internal(format!("Failed to set fuel: {}", e)))?;
        store.limiter(|data| data);
        let instance = wasmtime::Instance::new(&mut store, &module, &[])
            .map_err(|e| AppError::internal(format!("Failed to instantiate WASM module: {}", e)))?;
        if let Ok(func) = instance.get_typed_func::<(), i32>(&mut store, function) {
            let result = func
                .call(&mut store, ())
                .map_err(|e| AppError::internal(format!("WASM execution failed: {}", e)))?;
            return Ok(result.to_le_bytes().to_vec());
        }
        if let Ok(func) = instance.get_typed_func::<(), ()>(&mut store, function) {
            func.call(&mut store, ())
                .map_err(|e| AppError::internal(format!("WASM execution failed: {}", e)))?;
            return Ok(vec![]);
        }
        if let Ok(func) = instance.get_typed_func::<(i32, i32), i32>(&mut store, function) {
            let result = func
                .call(&mut store, (0, 0))
                .map_err(|e| AppError::internal(format!("WASM execution failed: {}", e)))?;
            return Ok(result.to_le_bytes().to_vec());
        }
        Err(AppError::internal(format!(
            "Function '{}' not found or unsupported signature",
            function
        )))
    }

    pub fn has_function(&self, wasm_bytes: &[u8], function: &str) -> bool {
        let module = match wasmtime::Module::new(&self.engine, wasm_bytes) {
            Ok(m) => m,
            Err(_) => return false,
        };
        let result = module.exports().any(|e| e.name() == function);
        result
    }
}

#[derive(Clone)]
pub struct PluginSandbox {
    plugins: Arc<Mutex<Vec<SandboxedPlugin>>>,
}

impl PluginSandbox {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn call_lifecycle_hook(&self, wasm_bytes: &[u8], config: &PluginConfig, hook: &str) {
        #[cfg(feature = "wasm")]
        {
            let wasm_owned = wasm_bytes.to_vec();
            let config_owned = config.clone();
            let hook_owned = hook.to_string();
            let sandbox = match WasmSandbox::new(&config_owned) {
                Ok(s) => s,
                Err(_) => return,
            };
            if sandbox.has_function(&wasm_owned, &hook_owned) {
                let _ = tokio::task::spawn_blocking(move || {
                    sandbox.execute(&wasm_owned, &hook_owned, 100_000)
                })
                .await;
            }
        }
        #[cfg(not(feature = "wasm"))]
        {
            let _ = (wasm_bytes, config, hook); // 无 WASM 引擎：生命周期钩子为空操作
        }
    }

    pub async fn load_plugin(
        &self,
        id: &str,
        wasm_bytes: Vec<u8>,
        config: Option<PluginConfig>,
    ) -> Result<SandboxedPlugin, AppError> {
        let cfg = config.unwrap_or_default();
        #[cfg(feature = "wasm")]
        {
            let sandbox = WasmSandbox::new(&cfg)?;
            let _module = wasmtime::Module::new(&sandbox.engine, &wasm_bytes)
                .map_err(|e| AppError::BadRequest(format!("Invalid WASM module: {}", e)))?;
        }
        // 计算并存储 WASM 哈希（恢复/安装时用于完整性校验）
        let mut hasher = Sha256::new();
        hasher.update(&wasm_bytes);
        let hash = format!("{:x}", hasher.finalize());

        let plugin = SandboxedPlugin {
            id: id.to_string(),
            status: PluginStatus::Loaded,
            wasm_bytes: wasm_bytes.clone(),
            wasm_hash: hash,
            config: cfg.clone(),
            metrics: PluginMetrics::default(),
            loaded_at: Utc::now(),
            last_executed_at: None,
            last_error: None,
        };

        let mut plugins = self.plugins.lock().await;
        plugins.retain(|p| p.id != id);
        plugins.push(plugin.clone());
        drop(plugins);

        self.call_lifecycle_hook(&wasm_bytes, &cfg, "on_load").await;

        Ok(plugin)
    }

    pub async fn reload_plugin(
        &self,
        id: &str,
        new_wasm_bytes: Vec<u8>,
        new_config: Option<PluginConfig>,
        expected_hash: Option<&str>,
    ) -> Result<SandboxedPlugin, AppError> {
        // 重载前强制校验哈希（若提供了期望值）
        verify_wasm_hash(&new_wasm_bytes, expected_hash)?;
        let cfg = new_config.unwrap_or_default();
        #[cfg(feature = "wasm")]
        {
            let sandbox = WasmSandbox::new(&cfg)?;
            let _module = wasmtime::Module::new(&sandbox.engine, &new_wasm_bytes)
                .map_err(|e| AppError::BadRequest(format!("Invalid WASM module: {}", e)))?;
        }

        let mut plugins = self.plugins.lock().await;
        let plugin = plugins
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| AppError::NotFound(format!("Plugin {} not found", id)))?;

        let old_bytes = plugin.wasm_bytes.clone();
        let old_config = plugin.config.clone();

        plugin.wasm_bytes = new_wasm_bytes.clone();
        plugin.config = cfg.clone();
        plugin.status = PluginStatus::Loaded;

        #[cfg(feature = "wasm")]
        if let Err(e) = {
            let sandbox = WasmSandbox::new(&cfg)?;
            sandbox.execute(&new_wasm_bytes, "on_reload", 100_000)
        } {
            plugin.wasm_bytes = old_bytes;
            plugin.config = old_config;
            plugin.status = PluginStatus::Error(format!("Reload hook failed: {}", e));
            return Err(AppError::internal(format!(
                "Plugin reload hook 'on_reload' failed: {}",
                e
            )));
        }
        #[cfg(not(feature = "wasm"))]
        let _ = (old_bytes, old_config);

        Ok(plugin.clone())
    }

    pub async fn execute_plugin(
        &self,
        id: &str,
        function: &str,
        _args: Option<Vec<i32>>,
    ) -> Result<ExecutionResult, AppError> {
        // feature `wasm` 关闭时无法执行字节码，返回稳定错误
        #[cfg(not(feature = "wasm"))]
        {
            let _ = (id, function);
            return Err(AppError::ServiceUnavailable(
                "WASM execution disabled (feature `wasm` is off)".to_string(),
            ));
        }
        #[cfg(feature = "wasm")]
        {
            let mut plugins = self.plugins.lock().await;
            let plugin = plugins
                .iter_mut()
                .find(|p| p.id == id)
                .ok_or_else(|| AppError::NotFound(format!("Plugin {} not found", id)))?;
            if plugin.status == PluginStatus::Disabled {
                return Err(AppError::BadRequest(format!("Plugin {} is disabled", id)));
            }
            plugin.status = PluginStatus::Running;
            let config = plugin.config.clone();
            let wasm_bytes = plugin.wasm_bytes.clone();
            let function_name = function.to_string();
            drop(plugins);

            let sandbox = WasmSandbox::new(&config)?;
            let fuel_limit = 1_000_000;
            let start = std::time::Instant::now();
            let result = tokio::task::spawn_blocking(move || {
                sandbox.execute(&wasm_bytes, &function_name, fuel_limit)
            })
            .await
            .map_err(|e| AppError::internal(format!("WASM task failed: {}", e)))?;

            let elapsed_ms = start.elapsed().as_millis() as u64;

            let mut plugins = self.plugins.lock().await;
            if let Some(plugin) = plugins.iter_mut().find(|p| p.id == id) {
                plugin.status = match &result {
                    Ok(_) => PluginStatus::Loaded,
                    Err(e) => PluginStatus::Error(e.to_string()),
                };
                plugin.last_executed_at = Some(Utc::now());
                plugin.metrics.total_executions += 1;
                plugin.metrics.last_execution_ms = elapsed_ms;
                if elapsed_ms > plugin.metrics.max_execution_ms {
                    plugin.metrics.max_execution_ms = elapsed_ms;
                }
                if elapsed_ms < plugin.metrics.min_execution_ms {
                    plugin.metrics.min_execution_ms = elapsed_ms;
                }
                plugin.metrics.avg_execution_ms = if plugin.metrics.successful_executions
                    + plugin.metrics.failed_executions
                    > 0
                {
                    (plugin.metrics.avg_execution_ms * (plugin.metrics.total_executions - 1) as f64
                        + elapsed_ms as f64)
                        / plugin.metrics.total_executions as f64
                } else {
                    elapsed_ms as f64
                };
                match &result {
                    Ok(_) => {
                        plugin.metrics.successful_executions += 1;
                        plugin.last_error = None;
                    }
                    Err(e) => {
                        plugin.metrics.failed_executions += 1;
                        plugin.last_error = Some(e.to_string());
                    }
                }
            }

            match result {
                Ok(output) => Ok(ExecutionResult {
                    output,
                    execution_ms: elapsed_ms,
                }),
                Err(e) => Err(e),
            }
        }
    }

    pub async fn get_plugin_metrics(&self, id: &str) -> Result<PluginMetrics, AppError> {
        let plugins = self.plugins.lock().await;
        let plugin = plugins
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| AppError::NotFound(format!("Plugin {} not found", id)))?;
        Ok(plugin.metrics.clone())
    }

    pub async fn list_plugins_metrics(&self) -> Vec<(String, PluginMetrics)> {
        let plugins = self.plugins.lock().await;
        plugins
            .iter()
            .map(|p| (p.id.clone(), p.metrics.clone()))
            .collect()
    }

    pub async fn set_plugin_setting(
        &self,
        id: &str,
        key: &str,
        value: &str,
    ) -> Result<(), AppError> {
        let mut plugins = self.plugins.lock().await;
        let plugin = plugins
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| AppError::NotFound(format!("Plugin {} not found", id)))?;
        plugin
            .config
            .settings
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    pub async fn get_plugin_setting(
        &self,
        id: &str,
        key: &str,
    ) -> Result<Option<String>, AppError> {
        let plugins = self.plugins.lock().await;
        let plugin = plugins
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| AppError::NotFound(format!("Plugin {} not found", id)))?;
        Ok(plugin.config.settings.get(key).cloned())
    }

    pub async fn list_plugin_settings(
        &self,
        id: &str,
    ) -> Result<HashMap<String, String>, AppError> {
        let plugins = self.plugins.lock().await;
        let plugin = plugins
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| AppError::NotFound(format!("Plugin {} not found", id)))?;
        Ok(plugin.config.settings.clone())
    }

    pub async fn reset_plugin_metrics(&self, id: &str) -> Result<(), AppError> {
        let mut plugins = self.plugins.lock().await;
        let plugin = plugins
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| AppError::NotFound(format!("Plugin {} not found", id)))?;
        plugin.metrics = PluginMetrics::default();
        Ok(())
    }

    pub async fn unload_plugin(&self, id: &str) -> Result<SandboxedPlugin, AppError> {
        let wasm_bytes;
        let config;
        {
            let plugins = self.plugins.lock().await;
            let plugin = plugins
                .iter()
                .find(|p| p.id == id)
                .ok_or_else(|| AppError::NotFound(format!("Plugin {} not found", id)))?;
            wasm_bytes = plugin.wasm_bytes.clone();
            config = plugin.config.clone();
        }

        self.call_lifecycle_hook(&wasm_bytes, &config, "on_unload")
            .await;

        let mut plugins = self.plugins.lock().await;
        let pos = plugins
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| AppError::NotFound(format!("Plugin {} not found", id)))?;
        let mut plugin = plugins.remove(pos);
        plugin.status = PluginStatus::Unloaded;
        Ok(plugin)
    }

    pub async fn enable_plugin(&self, id: &str) -> Result<SandboxedPlugin, AppError> {
        let mut plugins = self.plugins.lock().await;
        let plugin = plugins
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| AppError::NotFound(format!("Plugin {} not found", id)))?;
        let was_bytes = plugin.wasm_bytes.clone();
        let config = plugin.config.clone();
        if plugin.status == PluginStatus::Disabled {
            plugin.status = PluginStatus::Loaded;
        }
        drop(plugins);
        self.call_lifecycle_hook(&was_bytes, &config, "on_enable")
            .await;
        let plugins = self.plugins.lock().await;
        Ok(plugins.iter().find(|p| p.id == id).cloned().unwrap())
    }

    pub async fn disable_plugin(&self, id: &str) -> Result<SandboxedPlugin, AppError> {
        let wasm_bytes;
        let config;
        {
            let plugins = self.plugins.lock().await;
            let plugin = plugins
                .iter()
                .find(|p| p.id == id)
                .ok_or_else(|| AppError::NotFound(format!("Plugin {} not found", id)))?;
            wasm_bytes = plugin.wasm_bytes.clone();
            config = plugin.config.clone();
        }

        self.call_lifecycle_hook(&wasm_bytes, &config, "on_disable")
            .await;

        let mut plugins = self.plugins.lock().await;
        let plugin = plugins
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| AppError::NotFound(format!("Plugin {} not found", id)))?;
        plugin.status = PluginStatus::Disabled;
        Ok(plugin.clone())
    }

    pub async fn get_plugin(&self, id: &str) -> Result<SandboxedPlugin, AppError> {
        let plugins = self.plugins.lock().await;
        plugins
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("Plugin {} not found", id)))
    }

    pub async fn list_plugins(&self) -> Vec<SandboxedPlugin> {
        let plugins = self.plugins.lock().await;
        plugins.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub output: Vec<u8>,
    pub execution_ms: u64,
}

impl ExecutionResult {
    pub fn output_as_string(&self) -> String {
        String::from_utf8_lossy(&self.output).to_string()
    }
    pub fn output_as_i32(&self) -> Option<i32> {
        if self.output.len() >= 4 {
            Some(i32::from_le_bytes([
                self.output[0],
                self.output[1],
                self.output[2],
                self.output[3],
            ]))
        } else {
            None
        }
    }
}
impl Default for PluginSandbox {
    fn default() -> Self {
        Self::new()
    }
}
