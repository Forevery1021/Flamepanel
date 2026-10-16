//! LLM Provider 实现（M-AI-3）：OpenAI 兼容 `/chat/completions` 与 Ollama `/api/chat`。
//!
//! 两者都走非流式 JSON 请求，复用已有的 `reqwest`（rustls），不新增依赖。
//! 约束：
//! - 不跟随重定向（服务地址由管理员填写，重定向到别处属于异常）；
//! - 响应体上限 4 MiB；超时取配置 `timeout_secs`；
//! - 错误信息只含状态码与截断的响应片段，并抹掉可能被回显的 API Key。
use crate::core::error::AppError;
use crate::domain::assistant::{
    AssistantConfig, ChatMessage, ChatRole, LlmProvider, LlmProviderKind, LlmReply, ToolCall,
    ToolSpec,
};
use serde_json::{json, Map, Value};
use std::sync::Arc;
use std::time::Duration;

const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

/// 组合根注入 application 层的工厂。
pub fn factory() -> crate::domain::assistant::LlmProviderFactory {
    Arc::new(build_provider)
}

/// 按配置构建对应协议的 Provider。
pub fn build_provider(cfg: &AssistantConfig) -> Arc<dyn LlmProvider> {
    let http = HttpCore::new(cfg);
    match cfg.provider {
        LlmProviderKind::Openai => Arc::new(OpenAiCompatProvider {
            http,
            model: cfg.model.trim().to_string(),
            temperature: cfg.temperature,
        }),
        LlmProviderKind::Ollama => Arc::new(OllamaProvider {
            http,
            model: cfg.model.trim().to_string(),
            temperature: cfg.temperature,
        }),
    }
}

struct HttpCore {
    client: Result<reqwest::Client, String>,
    base: String,
    api_key: String,
}

impl HttpCore {
    fn new(cfg: &AssistantConfig) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(cfg.timeout_secs))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("FlamePanel/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| format!("HTTP 客户端初始化失败: {e}"));
        Self {
            client,
            base: cfg.effective_base_url(),
            api_key: cfg.api_key.clone(),
        }
    }

    fn scrub(&self, s: &str) -> String {
        let clipped: String = s.chars().take(300).collect();
        if self.api_key.len() >= 4 {
            clipped.replace(&self.api_key, "***")
        } else {
            clipped
        }
    }

    async fn send(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, AppError> {
        let client = self
            .client
            .as_ref()
            .map_err(|e| AppError::internal(e.clone()))?;
        let url = format!("{}{}", self.base, path);
        let mut req = client.request(method, &url);
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req.send().await.map_err(|e| {
            let kind = if e.is_timeout() {
                "请求超时"
            } else if e.is_connect() {
                "无法连接"
            } else {
                "请求失败"
            };
            AppError::ServiceUnavailable(format!("LLM 服务{kind}（{}）", self.base))
        })?;
        let status = resp.status();
        let bytes = read_capped(resp).await?;
        let text = String::from_utf8_lossy(&bytes);
        if !status.is_success() {
            return Err(AppError::ServiceUnavailable(format!(
                "LLM 服务返回 HTTP {}：{}",
                status.as_u16(),
                self.scrub(&text)
            )));
        }
        serde_json::from_slice(&bytes).map_err(|_| {
            AppError::ServiceUnavailable(format!("LLM 服务返回的不是 JSON：{}", self.scrub(&text)))
        })
    }
}

async fn read_capped(mut resp: reqwest::Response) -> Result<Vec<u8>, AppError> {
    let mut buf = Vec::new();
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|_| AppError::ServiceUnavailable("读取 LLM 响应失败".into()))?
    {
        if buf.len() + chunk.len() > MAX_RESPONSE_BYTES {
            return Err(AppError::ServiceUnavailable(
                "LLM 响应超过 4 MiB 上限".into(),
            ));
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(buf)
}

fn role_str(r: ChatRole) -> &'static str {
    match r {
        ChatRole::System => "system",
        ChatRole::User => "user",
        ChatRole::Assistant => "assistant",
        ChatRole::Tool => "tool",
    }
}

fn tools_wire(tools: &[ToolSpec]) -> Vec<Value> {
    tools
        .iter()
        .map(|t| {
            json!({
                "type": "function",
                "function": {"name": t.name, "description": t.description, "parameters": t.parameters},
            })
        })
        .collect()
}

/// 解析工具参数：OpenAI 为 JSON 字符串，Ollama 为对象；非法 JSON 保留原文交给工具层报错。
fn parse_arguments(v: Option<&Value>) -> Value {
    match v {
        Some(Value::String(s)) if s.trim().is_empty() => Value::Object(Map::new()),
        Some(Value::String(s)) => serde_json::from_str(s).unwrap_or_else(|_| json!({"_raw": s})),
        Some(Value::Null) | None => Value::Object(Map::new()),
        Some(v) => v.clone(),
    }
}

fn parse_tool_calls(v: Option<&Value>, id_prefix: &str) -> Vec<ToolCall> {
    let Some(Value::Array(arr)) = v else {
        return Vec::new();
    };
    arr.iter()
        .enumerate()
        .filter_map(|(i, c)| {
            let f = c.get("function")?;
            let name = f.get("name")?.as_str()?.to_string();
            let id = c
                .get("id")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| format!("{id_prefix}{i}"));
            Some(ToolCall {
                id,
                name,
                arguments: parse_arguments(f.get("arguments")),
            })
        })
        .collect()
}

// ─── OpenAI 兼容 ────────────────────────────────────────────────────────

pub struct OpenAiCompatProvider {
    http: HttpCore,
    model: String,
    temperature: f32,
}

/// OpenAI 兼容请求体（纯函数，便于单测）。
pub fn openai_request(
    model: &str,
    temperature: f32,
    messages: &[ChatMessage],
    tools: &[ToolSpec],
) -> Value {
    let msgs: Vec<Value> = messages
        .iter()
        .map(|m| {
            let mut o = json!({"role": role_str(m.role), "content": m.content});
            if !m.tool_calls.is_empty() {
                o["tool_calls"] = Value::Array(
                    m.tool_calls
                        .iter()
                        .map(|c| {
                            json!({
                                "id": c.id,
                                "type": "function",
                                "function": {"name": c.name, "arguments": c.arguments.to_string()},
                            })
                        })
                        .collect(),
                );
            }
            if let Some(id) = &m.tool_call_id {
                o["tool_call_id"] = json!(id);
            }
            o
        })
        .collect();
    let mut body =
        json!({"model": model, "messages": msgs, "temperature": temperature, "stream": false});
    if !tools.is_empty() {
        body["tools"] = Value::Array(tools_wire(tools));
    }
    body
}

/// 解析 OpenAI 兼容响应。
pub fn openai_reply(v: &Value) -> Result<LlmReply, AppError> {
    let msg = v
        .pointer("/choices/0/message")
        .ok_or_else(|| AppError::ServiceUnavailable("LLM 响应缺少 choices[0].message".into()))?;
    Ok(LlmReply {
        content: msg
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        tool_calls: parse_tool_calls(msg.get("tool_calls"), "call_"),
    })
}

#[async_trait::async_trait]
impl LlmProvider for OpenAiCompatProvider {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolSpec],
    ) -> Result<LlmReply, AppError> {
        let body = openai_request(&self.model, self.temperature, messages, tools);
        let v = self
            .http
            .send(reqwest::Method::POST, "/chat/completions", Some(&body))
            .await?;
        openai_reply(&v)
    }

    async fn list_models(&self) -> Result<Vec<String>, AppError> {
        let v = self
            .http
            .send(reqwest::Method::GET, "/models", None)
            .await?;
        Ok(v.get("data")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|m| m.get("id").and_then(Value::as_str).map(str::to_string))
                    .collect()
            })
            .unwrap_or_default())
    }
}

// ─── Ollama ─────────────────────────────────────────────────────────────

pub struct OllamaProvider {
    http: HttpCore,
    model: String,
    temperature: f32,
}

/// Ollama `/api/chat` 请求体。
pub fn ollama_request(
    model: &str,
    temperature: f32,
    messages: &[ChatMessage],
    tools: &[ToolSpec],
) -> Value {
    let msgs: Vec<Value> = messages
        .iter()
        .map(|m| {
            let mut o = json!({"role": role_str(m.role), "content": m.content});
            if !m.tool_calls.is_empty() {
                o["tool_calls"] = Value::Array(
                    m.tool_calls
                        .iter()
                        .map(|c| json!({"function": {"name": c.name, "arguments": c.arguments}}))
                        .collect(),
                );
            }
            if let Some(n) = &m.tool_name {
                o["tool_name"] = json!(n);
            }
            o
        })
        .collect();
    let mut body = json!({
        "model": model,
        "messages": msgs,
        "stream": false,
        "options": {"temperature": temperature},
    });
    if !tools.is_empty() {
        body["tools"] = Value::Array(tools_wire(tools));
    }
    body
}

/// 解析 Ollama 响应。
pub fn ollama_reply(v: &Value) -> Result<LlmReply, AppError> {
    let msg = v
        .get("message")
        .ok_or_else(|| AppError::ServiceUnavailable("Ollama 响应缺少 message".into()))?;
    Ok(LlmReply {
        content: msg
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        tool_calls: parse_tool_calls(msg.get("tool_calls"), "ollama_"),
    })
}

#[async_trait::async_trait]
impl LlmProvider for OllamaProvider {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolSpec],
    ) -> Result<LlmReply, AppError> {
        let body = ollama_request(&self.model, self.temperature, messages, tools);
        let v = self
            .http
            .send(reqwest::Method::POST, "/api/chat", Some(&body))
            .await?;
        ollama_reply(&v)
    }

    async fn list_models(&self) -> Result<Vec<String>, AppError> {
        let v = self
            .http
            .send(reqwest::Method::GET, "/api/tags", None)
            .await?;
        Ok(v.get("models")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|m| m.get("name").and_then(Value::as_str).map(str::to_string))
                    .collect()
            })
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, routing::post, Json, Router};

    fn spec() -> ToolSpec {
        ToolSpec {
            name: "docker_list".into(),
            description: "列出容器".into(),
            parameters: json!({"type": "object", "properties": {}}),
        }
    }

    fn history() -> Vec<ChatMessage> {
        let call = ToolCall {
            id: "c1".into(),
            name: "docker_list".into(),
            arguments: json!({"all": true}),
        };
        vec![
            ChatMessage::text(ChatRole::System, "sys"),
            ChatMessage::text(ChatRole::User, "有哪些容器"),
            ChatMessage {
                role: ChatRole::Assistant,
                content: String::new(),
                tool_calls: vec![call.clone()],
                tool_call_id: None,
                tool_name: None,
            },
            ChatMessage::tool_result(&call, "[]"),
        ]
    }

    #[test]
    fn openai_wire_format() {
        let b = openai_request("gpt-4o-mini", 0.2, &history(), &[spec()]);
        assert_eq!(b["model"], "gpt-4o-mini");
        assert_eq!(
            b["messages"][2]["tool_calls"][0]["function"]["arguments"],
            "{\"all\":true}"
        );
        assert_eq!(b["messages"][3]["role"], "tool");
        assert_eq!(b["messages"][3]["tool_call_id"], "c1");
        assert_eq!(b["tools"][0]["function"]["name"], "docker_list");
        // 无工具时不带 tools 字段（部分兼容服务对空数组报错）
        assert!(openai_request("m", 0.0, &history(), &[])
            .get("tools")
            .is_none());
    }

    #[test]
    fn openai_reply_parses_string_arguments() {
        let r = openai_reply(&json!({"choices": [{"message": {"content": null, "tool_calls": [
            {"id": "x", "type": "function", "function": {"name": "docker_logs", "arguments": "{\"container\":\"web\"}"}},
            {"type": "function", "function": {"name": "system_status", "arguments": ""}},
            {"type": "function", "function": {"name": "bad", "arguments": "{oops"}},
        ]}}]}))
        .unwrap();
        assert_eq!(r.content, "");
        assert_eq!(r.tool_calls.len(), 3);
        assert_eq!(r.tool_calls[0].arguments, json!({"container": "web"}));
        assert_eq!(r.tool_calls[1].id, "call_1");
        assert_eq!(r.tool_calls[1].arguments, json!({}));
        assert_eq!(r.tool_calls[2].arguments, json!({"_raw": "{oops"}));
        assert!(openai_reply(&json!({"error": "x"})).is_err());
    }

    #[test]
    fn ollama_wire_format() {
        let b = ollama_request("qwen2.5", 0.1, &history(), &[spec()]);
        assert_eq!(b["stream"], false);
        assert_eq!(
            b["messages"][2]["tool_calls"][0]["function"]["arguments"],
            json!({"all": true})
        );
        assert_eq!(b["messages"][3]["tool_name"], "docker_list");
        let r = ollama_reply(
            &json!({"message": {"role": "assistant", "content": "", "tool_calls": [
                {"function": {"name": "system_status", "arguments": {}}}
            ]}}),
        )
        .unwrap();
        assert_eq!(r.tool_calls[0].id, "ollama_0");
        assert_eq!(r.tool_calls[0].name, "system_status");
    }

    async fn serve(app: Router) -> String {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = l.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(l, app).await;
        });
        format!("http://{addr}")
    }

    fn cfg(provider: LlmProviderKind, base: String, key: &str) -> AssistantConfig {
        AssistantConfig {
            enabled: true,
            provider,
            base_url: base,
            model: "m".into(),
            api_key: key.into(),
            timeout_secs: 5,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn openai_round_trip_sends_bearer() {
        let app = Router::new()
            .route(
                "/v1/chat/completions",
                post(|headers: axum::http::HeaderMap, Json(b): Json<Value>| async move {
                    let auth = headers
                        .get("authorization")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("")
                        .to_string();
                    Json(json!({"choices": [{"message": {"content": format!("{auth}|{}", b["model"].as_str().unwrap())}}]}))
                }),
            )
            .route("/v1/models", get(|| async { Json(json!({"data": [{"id": "a"}, {"id": "b"}]})) }));
        let base = serve(app).await;
        let p = build_provider(&cfg(
            LlmProviderKind::Openai,
            format!("{base}/v1"),
            "sk-test",
        ));
        let r = p
            .chat(&[ChatMessage::text(ChatRole::User, "hi")], &[])
            .await
            .unwrap();
        assert_eq!(r.content, "Bearer sk-test|m");
        assert_eq!(p.list_models().await.unwrap(), vec!["a", "b"]);
    }

    #[tokio::test]
    async fn ollama_round_trip_and_error_scrubs_key() {
        let app = Router::new()
            .route(
                "/api/chat",
                post(|| async { Json(json!({"message": {"content": "pong"}})) }),
            )
            .route(
                "/api/tags",
                get(|| async { Json(json!({"models": [{"name": "qwen2.5:7b"}]})) }),
            );
        let base = serve(app).await;
        let p = build_provider(&cfg(LlmProviderKind::Ollama, base.clone(), ""));
        assert_eq!(p.chat(&[], &[]).await.unwrap().content, "pong");
        assert_eq!(p.list_models().await.unwrap(), vec!["qwen2.5:7b"]);

        // 服务端回显密钥的错误响应：错误信息中必须抹掉
        let echo = Router::new().route(
            "/v1/chat/completions",
            post(|| async {
                (
                    axum::http::StatusCode::UNAUTHORIZED,
                    "Incorrect API key provided: sk-leaky-key-123",
                )
            }),
        );
        let base = serve(echo).await;
        let p = build_provider(&cfg(
            LlmProviderKind::Openai,
            format!("{base}/v1"),
            "sk-leaky-key-123",
        ));
        let e = p.chat(&[], &[]).await.unwrap_err().to_string();
        assert!(e.contains("401"), "{e}");
        assert!(!e.contains("sk-leaky-key-123"), "{e}");
    }

    #[tokio::test]
    async fn unreachable_and_redirect_are_errors() {
        // 端口 1 基本不会有服务监听
        let p = build_provider(&cfg(
            LlmProviderKind::Ollama,
            "http://127.0.0.1:1".into(),
            "",
        ));
        assert!(p.list_models().await.is_err());
        let app = Router::new().route(
            "/api/tags",
            get(|| async { axum::response::Redirect::temporary("http://169.254.169.254/") }),
        );
        let base = serve(app).await;
        let p = build_provider(&cfg(LlmProviderKind::Ollama, base, ""));
        let e = p.list_models().await.unwrap_err().to_string();
        assert!(e.contains("307"), "重定向不跟随，按非 2xx 报错：{e}");
    }
}
