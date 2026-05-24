use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

// ─── Tool definition ───────────────────────────────────────────────────────────

pub type ToolHandler = Arc<
    dyn Fn(serde_json::Value) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<String, String>> + Send>,
    > + Send + Sync,
>;

pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value, // JSON Schema
    pub handler: ToolHandler,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub name: String,
    #[serde(default)]
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResponse {
    pub name: String,
    pub result: String,
}

// ─── Tool Registry ─────────────────────────────────────────────────────────────

pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, tool: Tool) {
        self.tools.write().await.insert(tool.name.clone(), tool);
    }

    pub async fn list(&self) -> Vec<ToolInfo> {
        self.tools
            .read()
            .await
            .values()
            .map(|t| ToolInfo {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: t.parameters.clone(),
            })
            .collect()
    }

    pub async fn execute(&self, req: &ToolCallRequest) -> Result<String, String> {
        let tools = self.tools.read().await;
        let tool = tools
            .get(&req.name)
            .ok_or_else(|| format!("工具 '{}' 不存在", req.name))?;
        (tool.handler)(req.arguments.clone()).await
    }

    /// Convert registered tools to Ollama-compatible format.
    pub async fn to_ollama_tools(&self) -> Vec<serde_json::Value> {
        self.tools
            .read()
            .await
            .values()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            })
            .collect()
    }

    pub async fn get(&self, name: &str) -> Option<ToolInfo> {
        self.tools.read().await.get(name).map(|t| ToolInfo {
            name: t.name.clone(),
            description: t.description.clone(),
            parameters: t.parameters.clone(),
        })
    }
}

// ─── Built-in Tools ────────────────────────────────────────────────────────────

/// Register all built-in system tools.
pub async fn register_builtin_tools(registry: &ToolRegistry) {
    // ── get_system_info ──
    registry
        .register(Tool {
            name: "get_system_info".into(),
            description: "获取服务器系统信息，包括 CPU、内存、磁盘使用率和负载".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            handler: Arc::new(|_args| {
                Box::pin(async move {
                    let info = crate::application::SystemService::get_info();
                    Ok(serde_json::to_string_pretty(&info)
                        .unwrap_or_else(|e| format!("序列化失败: {e}")))
                })
            }),
        })
        .await;

    // ── get_gpu_info ──
    registry
        .register(Tool {
            name: "get_gpu_info".into(),
            description: "获取 GPU 信息，包括型号、温度、利用率、显存使用情况".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            handler: Arc::new(|_args| {
                Box::pin(async move {
                    let gpus = crate::application::SystemService::get_gpu_info();
                    if gpus.is_empty() {
                        Ok("未检测到 NVIDIA GPU".into())
                    } else {
                        Ok(serde_json::to_string_pretty(&gpus)
                            .unwrap_or_else(|e| format!("序列化失败: {e}")))
                    }
                })
            }),
        })
        .await;

    // ── list_docker_containers ──
    registry
        .register(Tool {
            name: "list_docker_containers".into(),
            description: "列出所有 Docker 容器，包括运行中和已停止的".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            handler: Arc::new(|_args| {
                Box::pin(async move {
                    let output = tokio::process::Command::new("docker")
                        .args(["ps", "-a", "--format", "{{.ID}}\t{{.Names}}\t{{.Status}}\t{{.Image}}"])
                        .output()
                        .await
                        .map_err(|e| format!("执行 docker 命令失败: {e}"))?;
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    if stdout.trim().is_empty() {
                        Ok("没有容器".into())
                    } else {
                        Ok(stdout)
                    }
                })
            }),
        })
        .await;

    // ── list_files ──
    registry
        .register(Tool {
            name: "list_files".into(),
            description: "列出指定目录下的文件和文件夹".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "要列出的目录路径，默认为 /"
                    }
                },
                "required": []
            }),
            handler: Arc::new(|args| {
                Box::pin(async move {
                    let path = args
                        .get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("/");
                    let mut entries = tokio::fs::read_dir(path).await
                        .map_err(|e| format!("读取目录失败: {e}"))?;
                    let mut result = Vec::new();
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let file_type = entry.file_type().await
                            .map(|ft| if ft.is_dir() { "DIR" } else { "FILE" })
                            .unwrap_or("???");
                        let size = entry.metadata().await
                            .map(|m| m.len())
                            .unwrap_or(0);
                        result.push(format!("{file_type}  {size:>10}  {name}"));
                    }
                    if result.is_empty() {
                        Ok("目录为空".into())
                    } else {
                        Ok(result.join("\n"))
                    }
                })
            }),
        })
        .await;

    // ── get_recent_logs ──
    registry
        .register(Tool {
            name: "get_recent_logs".into(),
            description: "获取面板最近的操作日志记录".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "count": {
                        "type": "integer",
                        "description": "获取的日志条数，默认 20"
                    }
                },
                "required": []
            }),
            handler: Arc::new(|_args| {
                Box::pin(async move {
                    // Return placeholder — actual DB access needs AppState
                    Ok("日志功能需要通过面板界面访问".into())
                })
            }),
        })
        .await;

    // ── run_shell_command ──
    registry
        .register(Tool {
            name: "run_shell_command".into(),
            description: "执行一个 Shell 命令并返回输出（请谨慎使用，避免执行危险命令）".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "要执行的 shell 命令"
                    }
                },
                "required": ["command"]
            }),
            handler: Arc::new(|args| {
                Box::pin(async move {
                    let cmd = args
                        .get("command")
                        .and_then(|v| v.as_str())
                        .ok_or("缺少 command 参数".to_string())?;

                    // Safety: restrict dangerous commands
                    let dangerous = ["rm -rf", "mkfs", "dd if", ":(){", "chmod 777 /"];
                    for pattern in &dangerous {
                        if cmd.contains(pattern) {
                            return Err(format!("危险命令已阻止: 包含 '{}'", pattern));
                        }
                    }

                    let output = tokio::process::Command::new("sh")
                        .args(["-c", cmd])
                        .output()
                        .await
                        .map_err(|e| format!("执行命令失败: {e}"))?;

                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let mut result = String::new();
                    if !stdout.trim().is_empty() {
                        result.push_str(&format!("[stdout]\n{stdout}"));
                    }
                    if !stderr.trim().is_empty() {
                        result.push_str(&format!("\n[stderr]\n{stderr}"));
                    }
                    if result.is_empty() {
                        result.push_str("(无输出)");
                    }
                    Ok(result)
                })
            }),
        })
        .await;

    // ── get_docker_container_logs ──
    registry
        .register(Tool {
            name: "get_docker_container_logs".into(),
            description: "获取指定 Docker 容器的最近日志".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "container_name": {
                        "type": "string",
                        "description": "容器名称或 ID"
                    }
                },
                "required": ["container_name"]
            }),
            handler: Arc::new(|args| {
                Box::pin(async move {
                    let name = args
                        .get("container_name")
                        .and_then(|v| v.as_str())
                        .ok_or("缺少 container_name 参数".to_string())?;

                    let output = tokio::process::Command::new("docker")
                        .args(["logs", "--tail", "100", name])
                        .output()
                        .await
                        .map_err(|e| format!("获取容器日志失败: {e}"))?;

                    Ok(String::from_utf8_lossy(&output.stdout).to_string())
                })
            }),
        })
        .await;
}
