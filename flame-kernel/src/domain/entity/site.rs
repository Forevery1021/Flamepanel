//! 网站、Web 服务器实例、证书、DNS 厂商账号、WAF。

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Website {
    pub id: i64,
    pub name: String,
    pub domain: String,
    pub root_path: String,
    pub status: String,
    pub node_id: i64,
    pub engine: String,
    pub ssl_enabled: bool,
    pub proxy_enabled: bool,
    pub proxy_pass: Option<String>,
    /// 站点类型：runtime_php | reverse_proxy | static | container_app（M-B 主线二 Stage A-6 · Doc/20 §5.5）
    #[serde(default = "default_site_type")]
    pub site_type: String,
    /// 绑定的运行时 ID（site_type = runtime_php 场景）
    #[serde(default)]
    pub runtime_id: Option<i64>,
    /// 绑定的项目 ID（site_type = reverse_proxy + 项目场景）
    #[serde(default)]
    pub project_id: Option<i64>,
    /// 已套用的伪静态模板名（Doc/20 §5.5 伪静态规则库；None = 未套用）。
    /// 生成站点配置时按该名解析模板并注入 vhost（套用即生效）。
    #[serde(default)]
    pub rewrite_preset: Option<String>,
    pub created_at: DateTime<Utc>,
    /// 乐观并发控制版本号：写入时需匹配当前值，冲突返回稳定 `CONFLICT`。
    #[serde(default)]
    pub resource_version: i64,
}

fn default_site_type() -> String {
    "static".to_string()
}

impl Website {
    /// 合法站点类型白名单（Doc/20 §5.5）
    pub fn is_valid_site_type(t: &str) -> bool {
        matches!(
            t,
            "runtime_php" | "reverse_proxy" | "static" | "container_app"
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebServerInstance {
    pub id: i64,
    pub engine: String,
    pub version: Option<String>,
    pub status: String,
    pub config_path: String,
    pub binary_path: Option<String>,
    pub port: i32,
    pub created_at: DateTime<Utc>,
    /// 乐观并发控制版本号：写入时需匹配当前值，冲突返回稳定 `CONFLICT`。
    #[serde(default)]
    pub resource_version: i64,
}

/// SSL 证书记录（M-B 主线三 §6.1 SSL 证书自动化 · Stage 1 数据层）。
///
/// 一条记录 = 一次签发/上传的证书（SAN 列表维度唯一）。ACME 账户私钥等
/// 敏感凭据不落本表——账户密钥持久化在 `data/acme/account.key`（0600），
/// DNS 厂商 API 凭据复用 `SecretCipher` 加密语义（落库仅密文）。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Certificate {
    pub id: i64,
    /// SAN 列表 JSON（主域名 + 备用域名/泛域名，如 `["example.com","*.example.com"]`）
    pub domains: String,
    /// 证书来源：letsencrypt | zerossl | self_signed | uploaded
    pub source: String,
    /// DNS-01 厂商：cloudflare | aliyun | tencent | huawei（HTTP-01/无 DNS 为 None）
    pub dns_provider: Option<String>,
    /// 是否自动续签
    pub auto_renew: bool,
    /// 证书文件路径（PEM 全链）
    pub cert_path: Option<String>,
    /// 私钥文件路径（0600）
    pub key_path: Option<String>,
    /// 到期时刻（PEM 解析得来，None = 未签发/待上传）
    pub expires_at: Option<DateTime<Utc>>,
    /// 最近一次续签/签发时刻
    pub last_renewed_at: Option<DateTime<Utc>>,
    /// 关联网站 ID（可选；解除网站绑定不删除证书记录）
    pub website_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for Certificate {
    fn default() -> Self {
        Self {
            id: 0,
            domains: "[]".to_string(),
            source: "letsencrypt".to_string(),
            dns_provider: None,
            auto_renew: true,
            cert_path: None,
            key_path: None,
            expires_at: None,
            last_renewed_at: None,
            website_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl Certificate {
    /// 合法来源白名单。
    pub fn is_valid_source(s: &str) -> bool {
        matches!(s, "letsencrypt" | "zerossl" | "self_signed" | "uploaded")
    }

    /// 合法 DNS-01 厂商白名单（None = 不使用 DNS-01）。
    pub fn is_valid_dns_provider(p: &str) -> bool {
        matches!(p, "cloudflare" | "aliyun" | "tencent" | "huawei")
    }

    /// 解析 SAN 列表 JSON 并校验形态（必须是字符串数组；服务层写入前统一调用）。
    pub fn domain_array(domains_json: &str) -> Result<Vec<String>, crate::core::error::AppError> {
        let list: Vec<String> = serde_json::from_str(domains_json).map_err(|_| {
            crate::core::error::AppError::ValidationError("domains 必须是 JSON 字符串数组".into())
        })?;
        Ok(list)
    }

    /// 解析 SAN 列表；空/非法 JSON 返回空表（防御性，写入时已校验）。
    pub fn domain_list(&self) -> Vec<String> {
        serde_json::from_str::<Vec<String>>(&self.domains).unwrap_or_default()
    }

    /// 主域名（SAN 第一项）。
    pub fn primary_domain(&self) -> Option<String> {
        self.domain_list().into_iter().next()
    }

    /// SAN 列表写入前校验：1–10 项，每项为合法域名（字母数字连字符点，含泛域名前缀），
    /// 全表去重后主域名唯一约束由 UNIQUE(domains) 保证。
    pub fn validate_domains(domains: &[String]) -> Result<(), crate::domain::error::DomainError> {
        if domains.is_empty() {
            return Err(crate::domain::error::DomainError::Validation(
                "域名列表不能为空".to_string(),
            ));
        }
        if domains.len() > 10 {
            return Err(crate::domain::error::DomainError::Validation(
                "域名列表最多 10 项".to_string(),
            ));
        }
        let re =
            regex::Regex::new(r"^(\*\.)?([a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}$")
                .expect("内建正则必合法");
        for d in domains {
            if !re.is_match(d) {
                return Err(crate::domain::error::DomainError::Validation(format!(
                    "非法域名: {}",
                    d
                )));
            }
        }
        Ok(())
    }

    /// 来源是否支持自动续签（仅 ACME：letsencrypt / zerossl；自签与上传证书无签发方可续）。
    pub fn supports_auto_renew(&self) -> bool {
        matches!(self.source.as_str(), "letsencrypt" | "zerossl")
    }

    /// 续签窗口起点：到期前 30 天（无到期时间为 None）。
    pub fn renewal_window_starts_at(&self) -> Option<DateTime<Utc>> {
        self.expires_at.map(|e| e - chrono::Duration::days(30))
    }

    /// 是否需要续签：未启用自动续签为 false；来源不支持续签为 false（S-10：老库
    /// `auto_renew` 列默认 1，上传证书曾被每 6 小时徒劳续签一次并告警）；无到期时间为 false；
    /// 距到期 ≤30 天为 true（对齐 Doc/20 §6.1 续签窗口）。
    pub fn needs_renewal(&self, now: DateTime<Utc>) -> bool {
        self.auto_renew
            && self.supports_auto_renew()
            && self.expires_at.is_some_and(|e| (e - now).num_days() <= 30)
    }
}

#[cfg(test)]
mod certificate_entity_tests {
    use super::*;

    #[test]
    fn validate_domains_rules() {
        assert!(Certificate::validate_domains(&["example.com".to_string()]).is_ok());
        assert!(Certificate::validate_domains(&["*.example.com".to_string()]).is_ok());
        assert!(Certificate::validate_domains(&["example.com".into(), "a.b.cn".into()]).is_ok());
        // 空 / 超量
        assert!(Certificate::validate_domains(&[]).is_err());
        let too_many: Vec<String> = (0..11).map(|i| format!("d{}.com", i)).collect();
        assert!(Certificate::validate_domains(&too_many).is_err());
        // 非法形态
        assert!(Certificate::validate_domains(&["-bad.com".to_string()]).is_err());
        assert!(Certificate::validate_domains(&["a b.com".to_string()]).is_err());
        assert!(Certificate::validate_domains(&["*.example.com/path".to_string()]).is_err());
        assert!(Certificate::validate_domains(&["http://x.com".to_string()]).is_err());
    }

    #[test]
    fn needs_renewal_window() {
        let mut c = Certificate::default();
        let now = Utc::now();
        // 未签发：不续签
        assert!(!c.needs_renewal(now));
        // 90 天后到期：不续签
        c.expires_at = Some(now + chrono::Duration::days(90));
        assert!(!c.needs_renewal(now));
        // 29 天后到期：续签
        c.expires_at = Some(now + chrono::Duration::days(29));
        assert!(c.needs_renewal(now));
        // 已过期：续签
        c.expires_at = Some(now - chrono::Duration::days(1));
        assert!(c.needs_renewal(now));
        // 关闭自动续签：不续签
        c.auto_renew = false;
        assert!(!c.needs_renewal(now));
    }

    #[test]
    fn unsupported_sources_never_need_renewal() {
        // S-10：老库 auto_renew 列默认 1——上传 / 自签证书即便开关为真也不进续签扫描
        let now = Utc::now();
        for (source, supported) in [
            ("letsencrypt", true),
            ("zerossl", true),
            ("self_signed", false),
            ("uploaded", false),
        ] {
            let c = Certificate {
                source: source.into(),
                auto_renew: true,
                expires_at: Some(now + chrono::Duration::days(3)),
                ..Certificate::default()
            };
            assert_eq!(c.supports_auto_renew(), supported, "{source}");
            assert_eq!(c.needs_renewal(now), supported, "{source}");
        }
        let c = Certificate {
            expires_at: Some(now + chrono::Duration::days(40)),
            ..Certificate::default()
        };
        assert_eq!(
            c.renewal_window_starts_at(),
            Some(now + chrono::Duration::days(10))
        );
        assert!(Certificate::default().renewal_window_starts_at().is_none());
    }

    #[test]
    fn whitelist_helpers() {
        assert!(Certificate::is_valid_source("letsencrypt"));
        assert!(Certificate::is_valid_source("uploaded"));
        assert!(!Certificate::is_valid_source("godaddy"));
        assert!(Certificate::is_valid_dns_provider("cloudflare"));
        assert!(Certificate::is_valid_dns_provider("aliyun"));
        assert!(!Certificate::is_valid_dns_provider("http"));
    }

    #[test]
    fn domain_list_defensive_parse() {
        let mut c = Certificate {
            domains: r#"["a.com","b.com"]"#.to_string(),
            ..Default::default()
        };
        assert_eq!(c.domain_list().len(), 2);
        c.domains = "not json".to_string();
        assert!(c.domain_list().is_empty());
    }
}

// ── DNS 厂商账号（主线三 §6.1 · Stage 4 · Doc/20）────────────────────

/// DNS 厂商账号（§6.1 Stage 4 · DNS-01 泛域名无人值守续签前置）。
///
/// 面板托管的第三方 DNS 厂商 API 凭据，用于 ACME DNS-01 验证写 TXT 记录。
/// `credentials_json` 为各厂商各自的凭据结构（API Token / AccessKey 对）序列化，
/// 落库前经 `SecretCipher`（AES-256-GCM）加密为 `enc:v1:` 密文（对齐备份账号语义），
/// 使用时透明解密，历史明文行静默兼容。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DnsProviderAccount {
    pub id: i64,
    /// 展示名称（UNIQUE 防重复命名）
    pub name: String,
    /// 厂商标识：cloudflare | aliyun | tencent | huawei（对齐 `Certificate::is_valid_dns_provider` 白名单）
    pub provider_type: String,
    /// 厂商凭据（JSON，结构由各 Provider 实现自解析；落库前加密）
    pub credentials_json: String,
    pub enabled: bool,
    /// 最近一次连通性测试：ok | failed（None 表示从未测试）
    pub last_test_status: Option<String>,
    pub last_test_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for DnsProviderAccount {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            provider_type: "cloudflare".to_string(),
            credentials_json: "{}".to_string(),
            enabled: true,
            last_test_status: None,
            last_test_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl DnsProviderAccount {
    /// 合法厂商标识白名单（与 `Certificate::is_valid_dns_provider` 对齐）。
    pub fn is_valid_provider_type(t: &str) -> bool {
        Certificate::is_valid_dns_provider(t)
    }

    /// 连通性测试是否通过
    pub fn is_test_ok(&self) -> bool {
        self.last_test_status.as_deref() == Some("ok")
    }
}

#[cfg(test)]
mod dns_provider_entity_tests {
    use super::*;

    #[test]
    fn provider_type_whitelist_aligns_with_certificate() {
        for p in ["cloudflare", "aliyun", "tencent", "huawei"] {
            assert!(DnsProviderAccount::is_valid_provider_type(p));
        }
        for bad in ["godaddy", "http", "", "CloudFlare"] {
            assert!(!DnsProviderAccount::is_valid_provider_type(bad));
        }
    }

    #[test]
    fn default_and_test_ok() {
        let d = DnsProviderAccount::default();
        assert_eq!(d.provider_type, "cloudflare");
        assert_eq!(d.credentials_json, "{}");
        assert!(d.enabled);
        assert!(!d.is_test_ok());
        d.is_test_ok();
    }
}

// ── WAF 基础防护（主线三 §6.2 · Doc/20）─────────────────────────────────

/// WAF 规则类型白名单（§6.2 首期规则集）：
/// cc（频率限制）/ sql_injection / xss / ip_blacklist / ip_whitelist /
/// ua_blacklist / ua_whitelist / path_block
pub const WAF_RULE_TYPES: [&str; 8] = [
    "cc",
    "sql_injection",
    "xss",
    "ip_blacklist",
    "ip_whitelist",
    "ua_blacklist",
    "ua_whitelist",
    "path_block",
];

/// WAF 匹配命中后的动作：log（仅告警，默认）| block（阻断）
pub const WAF_ACTIONS: [&str; 2] = ["log", "block"];

/// WAF 规则（§6.2 · 首期：CC 频率限制 / SQL 注入 / XSS 特征 / IP·UA 黑白名单 / 路径封禁）。
///
/// `website_id = 0` 表示全局规则（作用于全部站点）；`config_json` 为各规则类型
/// 自解析的参数结构（如 cc 的 `{ "threshold": 120, "window_secs": 60 }`）。
/// 默认动作 `log`：仅告警不阻断（§6.2 风险对策 6——误拦截优先可观测）。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WafRule {
    pub id: i64,
    /// 规则名称（展示用）
    pub name: String,
    /// 规则类型（`WAF_RULE_TYPES` 白名单）
    pub rule_type: String,
    /// 作用网站 ID（0 = 全局规则）
    pub website_id: i64,
    /// 匹配值：IP/CIDR、UA 子串、路径前缀或特征正则（按 rule_type 语义解析）
    pub pattern: String,
    /// 类型相关参数（JSON；cc 规则放 threshold/window_secs，其他类型可为 `{}`）
    pub config_json: String,
    /// 命中动作：log | block
    pub action: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for WafRule {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            rule_type: "ip_blacklist".to_string(),
            website_id: 0,
            pattern: String::new(),
            config_json: "{}".to_string(),
            action: "log".to_string(),
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl WafRule {
    /// 规则类型白名单校验。
    pub fn is_valid_rule_type(t: &str) -> bool {
        WAF_RULE_TYPES.contains(&t)
    }

    /// 动作白名单校验。
    pub fn is_valid_action(a: &str) -> bool {
        WAF_ACTIONS.contains(&a)
    }

    /// cc 规则参数解析：threshold（窗口内最大请求数，默认 120）与
    /// window_secs（统计窗口秒数，默认 60）；非法 JSON/字段回落默认值。
    pub fn cc_params(&self) -> (u32, u64) {
        let cfg: serde_json::Value = serde_json::from_str(&self.config_json).unwrap_or_default();
        let threshold = cfg
            .get("threshold")
            .and_then(|v| v.as_u64())
            .unwrap_or(120)
            .clamp(1, u32::MAX as u64) as u32;
        let window_secs = cfg
            .get("window_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(60);
        (threshold, window_secs.max(1))
    }
}

/// WAF 拦截日志（§6.2：网站维度记录，复用 operation_logs 查询/分页模式）。
///
/// 敏感请求详情（headers/body）不落库——只记来源 IP、命中规则、请求行摘要，
/// 防止日志成为敏感信息二次泄漏点。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WafLog {
    pub id: i64,
    /// 命中的网站 ID（0 = 全局规则命中）
    pub website_id: i64,
    /// 命中的规则 ID（规则删除后保留历史日志，页面显示规则名快照）
    pub rule_id: i64,
    /// 规则名称快照（规则删除后仍可读）
    pub rule_name: String,
    /// 规则类型快照
    pub rule_type: String,
    /// 命中动作快照：log | block
    pub action: String,
    /// 来源 IP
    pub client_ip: String,
    /// 请求方法 + 路径摘要（如 `GET /login?id=1' OR`；超长截断）
    pub request_summary: String,
    /// 命中的 UA（超长截断）
    pub user_agent: String,
    /// 命中时间
    pub created_at: DateTime<Utc>,
}

/// 截断工具：WAF 日志摘要字段超长截断（防日志表单字段膨胀）。
pub fn truncate_summary(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect()
    }
}

#[cfg(test)]
mod waf_entity_tests {
    use super::*;

    #[test]
    fn rule_type_and_action_whitelist() {
        for t in WAF_RULE_TYPES {
            assert!(WafRule::is_valid_rule_type(t));
        }
        assert!(!WafRule::is_valid_rule_type("rce"));
        assert!(!WafRule::is_valid_rule_type(""));

        assert!(WafRule::is_valid_action("log"));
        assert!(WafRule::is_valid_action("block"));
        assert!(!WafRule::is_valid_action("deny"));
    }

    #[test]
    fn cc_params_parse_and_defaults() {
        let mut r = WafRule {
            rule_type: "cc".into(),
            ..Default::default()
        };
        // 非法 JSON → 默认值
        r.config_json = "not-json".into();
        assert_eq!(r.cc_params(), (120, 60));
        // 合法参数
        r.config_json = r#"{"threshold":50,"window_secs":30}"#.into();
        assert_eq!(r.cc_params(), (50, 30));
        // threshold 下限钳制
        r.config_json = r#"{"threshold":0}"#.into();
        assert_eq!(r.cc_params().0, 1);
    }

    #[test]
    fn truncate_summary_works() {
        assert_eq!(truncate_summary("short", 10), "short");
        let long = "x".repeat(100);
        assert_eq!(truncate_summary(&long, 20).chars().count(), 20);
        // 多字节字符按字符数截断，不产生半个字符
        let cn = "中文字符串测试".repeat(10);
        assert_eq!(truncate_summary(&cn, 5), "中文字符串");
    }
}
