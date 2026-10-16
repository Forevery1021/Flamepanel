//! 面板内置 AI 助手（M-AI-3 · Doc/30 / Doc/32 §7）：配置模型与 LLM Provider 抽象。
//!
//! - **默认关闭**：未经管理员配置 Provider 时，对话接口返回 503。
//! - **Provider 抽象**：[`LlmProvider`] 只描述「带工具的一轮对话」与「列出模型」两件事，
//!   具体协议（OpenAI 兼容 `/chat/completions`、Ollama `/api/chat`）在 infrastructure 实现，
//!   application / api 层只依赖本 trait——测试可注入假 Provider，不需要真实模型。
//! - **密钥**：`api_key` 永不序列化到 API 响应；落库前由 application 层 AES-GCM 加密。
use crate::core::error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// 单条消息内容上限（字节）。
pub const MAX_MESSAGE_BYTES: usize = 32 * 1024;
/// 单次对话携带的历史消息条数上限。
pub const MAX_HISTORY: usize = 40;
/// 单轮对话工具调用步数上限（配置可调，硬上限）。
pub const MAX_STEPS_LIMIT: u8 = 12;

/// Provider 协议类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum LlmProviderKind {
    /// OpenAI 兼容协议（OpenAI / DeepSeek / 通义 / vLLM / LM Studio 等）
    #[default]
    Openai,
    /// Ollama 原生 `/api/chat`
    Ollama,
}

impl LlmProviderKind {
    /// 缺省服务地址。
    pub fn default_base_url(self) -> &'static str {
        match self {
            Self::Openai => "https://api.openai.com/v1",
            Self::Ollama => "http://127.0.0.1:11434",
        }
    }
}

/// 助手配置（持久化形态；`api_key` 为明文，仅在内核内存中存在）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantConfig {
    pub enabled: bool,
    pub provider: LlmProviderKind,
    pub base_url: String,
    pub model: String,
    #[serde(skip)]
    pub api_key: String,
    pub temperature: f32,
    pub max_steps: u8,
    pub timeout_secs: u64,
}

impl Default for AssistantConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: LlmProviderKind::Openai,
            base_url: String::new(),
            model: String::new(),
            api_key: String::new(),
            temperature: 0.2,
            max_steps: 6,
            timeout_secs: 60,
        }
    }
}

impl AssistantConfig {
    /// 生效的服务地址（留空取协议缺省，去掉末尾 `/`）。
    pub fn effective_base_url(&self) -> String {
        let b = self.base_url.trim();
        let b = if b.is_empty() {
            self.provider.default_base_url()
        } else {
            b
        };
        b.trim_end_matches('/').to_string()
    }

    /// 字段校验；错误信息面向管理员。
    pub fn validate(&self) -> Result<(), AppError> {
        let bad = |m: &str| Err(AppError::ValidationError(m.to_string()));
        let base = self.base_url.trim();
        if !base.is_empty() {
            let lower = base.to_ascii_lowercase();
            if !(lower.starts_with("http://") || lower.starts_with("https://")) {
                return bad("base_url 必须以 http:// 或 https:// 开头");
            }
            if base.len() > 512 || base.chars().any(|c| c.is_whitespace() || c.is_control()) {
                return bad("base_url 不合法（≤ 512 字符，不含空白）");
            }
            // 拒绝 `user:pass@host` 形式：凭据应填入 api_key，避免明文落库与日志
            let rest = &base[base.find("://").map(|i| i + 3).unwrap_or(0)..];
            let authority = rest.split('/').next().unwrap_or("");
            if authority.contains('@') {
                return bad("base_url 不能内嵌用户名 / 密码，请使用 API Key 字段");
            }
            if authority.is_empty() {
                return bad("base_url 缺少主机名");
            }
        }
        let model = self.model.trim();
        if self.enabled && model.is_empty() {
            return bad("启用助手前必须填写模型名称");
        }
        if model.len() > 128 || model.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return bad("模型名称不合法（≤ 128 字符，不含空白）");
        }
        if self.api_key.len() > 1024 || self.api_key.chars().any(char::is_control) {
            return bad("API Key 不合法");
        }
        if !(0.0..=2.0).contains(&self.temperature) || self.temperature.is_nan() {
            return bad("temperature 需在 0–2 之间");
        }
        if !(1..=MAX_STEPS_LIMIT).contains(&self.max_steps) {
            return bad("max_steps 需在 1–12 之间");
        }
        if !(5..=300).contains(&self.timeout_secs) {
            return bad("timeout_secs 需在 5–300 之间");
        }
        Ok(())
    }
}

/// 配置对外视图：不含密钥，只告知是否已设置。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AssistantConfigView {
    pub enabled: bool,
    pub provider: LlmProviderKind,
    /// 留空表示使用协议缺省地址
    pub base_url: String,
    /// 实际生效的地址
    pub effective_base_url: String,
    pub model: String,
    pub has_api_key: bool,
    pub temperature: f32,
    pub max_steps: u8,
    pub timeout_secs: u64,
}

impl From<&AssistantConfig> for AssistantConfigView {
    fn from(c: &AssistantConfig) -> Self {
        Self {
            enabled: c.enabled,
            provider: c.provider,
            base_url: c.base_url.clone(),
            effective_base_url: c.effective_base_url(),
            model: c.model.clone(),
            has_api_key: !c.api_key.is_empty(),
            temperature: c.temperature,
            max_steps: c.max_steps,
            timeout_secs: c.timeout_secs,
        }
    }
}

/// 配置更新（字段均可选，缺省保持原值）。
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct AssistantConfigPatch {
    pub enabled: Option<bool>,
    pub provider: Option<LlmProviderKind>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    /// 缺省 = 保留原密钥；空字符串 = 清除；其他 = 替换
    pub api_key: Option<String>,
    pub temperature: Option<f32>,
    pub max_steps: Option<u8>,
    pub timeout_secs: Option<u64>,
}

impl AssistantConfig {
    /// 应用补丁（不校验，调用方随后调用 [`AssistantConfig::validate`]）。
    pub fn apply(&mut self, p: AssistantConfigPatch) {
        if let Some(v) = p.enabled {
            self.enabled = v;
        }
        if let Some(v) = p.provider {
            self.provider = v;
        }
        if let Some(v) = p.base_url {
            self.base_url = v.trim().to_string();
        }
        if let Some(v) = p.model {
            self.model = v.trim().to_string();
        }
        if let Some(v) = p.api_key {
            self.api_key = v.trim().to_string();
        }
        if let Some(v) = p.temperature {
            self.temperature = v;
        }
        if let Some(v) = p.max_steps {
            self.max_steps = v;
        }
        if let Some(v) = p.timeout_secs {
            self.timeout_secs = v;
        }
    }
}

/// 对话角色。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

/// 模型发起的一次工具调用。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// 调用 ID（Ollama 不返回时由内核生成）
    pub id: String,
    pub name: String,
    /// 参数对象（已从 OpenAI 的 JSON 字符串形式解析）
    pub arguments: Value,
}

/// Provider 层的对话消息。
#[derive(Debug, Clone, PartialEq)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    /// 仅 assistant 消息：本轮发起的工具调用
    pub tool_calls: Vec<ToolCall>,
    /// 仅 tool 消息：对应的调用 ID 与工具名
    pub tool_call_id: Option<String>,
    pub tool_name: Option<String>,
}

impl ChatMessage {
    pub fn text(role: ChatRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
            tool_name: None,
        }
    }

    pub fn tool_result(call: &ToolCall, content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Tool,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: Some(call.id.clone()),
            tool_name: Some(call.name.clone()),
        }
    }
}

/// 提供给模型的工具描述（JSON Schema 参数）。
#[derive(Debug, Clone, PartialEq)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// 模型一轮回复。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LlmReply {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
}

/// LLM Provider 抽象。实现必须：超时由调用方配置、错误信息不回显密钥。
#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    /// 一轮对话（非流式）。`tools` 为空时模型只能回复文本。
    async fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolSpec],
    ) -> Result<LlmReply, AppError>;
    /// 列出可用模型（用于「测试连接」）。
    async fn list_models(&self) -> Result<Vec<String>, AppError>;
}

/// 按配置构建 Provider（由组合根注入 infrastructure 实现，application 层不依赖 HTTP 客户端）。
pub type LlmProviderFactory =
    std::sync::Arc<dyn Fn(&AssistantConfig) -> std::sync::Arc<dyn LlmProvider> + Send + Sync>;

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> AssistantConfig {
        AssistantConfig {
            enabled: true,
            model: "qwen2.5:7b".into(),
            ..Default::default()
        }
    }

    #[test]
    fn default_is_disabled_and_valid() {
        let c = AssistantConfig::default();
        assert!(!c.enabled);
        c.validate().unwrap();
    }

    #[test]
    fn effective_base_url_falls_back_per_provider() {
        let mut c = cfg();
        assert_eq!(c.effective_base_url(), "https://api.openai.com/v1");
        c.provider = LlmProviderKind::Ollama;
        assert_eq!(c.effective_base_url(), "http://127.0.0.1:11434");
        c.base_url = " http://10.0.0.5:11434/ ".into();
        assert_eq!(c.effective_base_url(), "http://10.0.0.5:11434");
    }

    #[test]
    fn validate_rejects_bad_fields() {
        let cases: [fn(&mut AssistantConfig); 12] = [
            |c| c.base_url = "ftp://x".into(),
            |c| c.base_url = "http://user:pw@host/v1".into(),
            |c| c.base_url = "http://".into(),
            |c| c.base_url = "http://a b".into(),
            |c| c.model = String::new(),
            |c| c.model = "a b".into(),
            |c| c.temperature = 3.0,
            |c| c.temperature = f32::NAN,
            |c| c.max_steps = 0,
            |c| c.max_steps = 13,
            |c| c.timeout_secs = 1,
            |c| c.api_key = "a\nb".into(),
        ];
        for (i, f) in cases.iter().enumerate() {
            let mut c = cfg();
            f(&mut c);
            assert!(c.validate().is_err(), "case {i} should fail");
        }
        cfg().validate().unwrap();
    }

    #[test]
    fn view_hides_key() {
        let mut c = cfg();
        c.api_key = "sk-secret".into();
        let v = AssistantConfigView::from(&c);
        assert!(v.has_api_key);
        let s = serde_json::to_string(&v).unwrap();
        assert!(!s.contains("sk-secret"));
        // 持久化形态同样不含密钥（密钥单独加密存储）
        assert!(!serde_json::to_string(&c).unwrap().contains("sk-secret"));
    }
}
