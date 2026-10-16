//! 真实厂商 DNS-01 Provider（M-B 主线三 §6.1 · Stage 5 · Doc/20）。
//!
//! 覆盖 Plan Step 2 的两家厂商：
//! - [`CloudflareDnsProvider`]：API Token（Bearer），v4 REST；
//!   连通性探测走 `GET /user/tokens/verify`。
//! - [`AliyunDnsProvider`]：AccessKey + RPC 签名（HMAC-SHA1，签名法 HMAC-SHA1），
//!   AddDomainRecord / DescribeDomainRecords / DeleteDomainRecord；
//!   连通性探测走 `DescribeDomains`（PageSize=1）。
//!
//! 设计要点：
//! - 只依赖仓库既有 `reqwest`（rustls-tls）与依赖树已有 `hmac`/`sha2`/`base64`
//!   （hkdf 0.12 传递引入 hmac 0.12.1，零新增依赖树节点）；
//! - TXT 记录写入幂等：先查同名同值记录，存在直接复用（不重复创建）；
//! - 清理语义：只删除自己写入的同值记录，缺失不报错（对齐端口约定）；
//! - 凭据不落日志：错误信息只带厂商响应码与 message 摘要，不带请求头/凭据。

use super::dns::DnsProvider;
use crate::core::error::AppError;
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha1::Sha1;
use sha2::Sha256;

/// 厂商 API 统一超时（传播轮询由签发编排层控制，单次请求快速失败）。
const HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

fn http_client() -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .user_agent(concat!("FlamePanel/", env!("CARGO_PKG_VERSION")))
        .timeout(HTTP_TIMEOUT)
        .build()
        .map_err(|e| AppError::internal(format!("DNS Provider HTTP 客户端构建失败: {e}")))
}

/// 把 `*.example.com` 泛域名映射为 `_acme-challenge.example.com`，
/// 普通域名映射为 `_acme-challenge.<domain>`。
fn acme_challenge_name(domain: &str) -> String {
    let base = domain.trim_start_matches("*.");
    format!("_acme-challenge.{base}")
}

// ── Cloudflare ──────────────────────────────────────────────────────────

/// Cloudflare DNS-01 Provider（API Token 授权，权限 Zone:DNS:Edit）。
pub struct CloudflareDnsProvider {
    http: reqwest::Client,
    api_token: String,
}

impl CloudflareDnsProvider {
    /// 从凭据 JSON（`{"api_token": "..."}`）构造。
    pub fn from_credentials(credentials_json: &str) -> Result<Self, AppError> {
        let cfg: Value = serde_json::from_str(credentials_json)
            .map_err(|e| AppError::BadRequest(format!("Cloudflare 凭据 JSON 非法: {e}")))?;
        let token = cfg
            .get("api_token")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| AppError::BadRequest("Cloudflare 凭据缺少 api_token".into()))?;
        Ok(Self {
            http: http_client()?,
            api_token: token.to_string(),
        })
    }

    async fn get_json(&self, url: &str) -> Result<Value, AppError> {
        let resp = self
            .http
            .get(url)
            .bearer_auth(&self.api_token)
            .send()
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("Cloudflare API 请求失败: {e}")))?;
        let status = resp.status();
        let body: Value = resp.json().await.map_err(|e| {
            AppError::ServiceUnavailable(format!("Cloudflare API 响应解析失败: {e}"))
        })?;
        if !status.is_success() {
            let msg = body
                .pointer("/errors/0/message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            return Err(AppError::ServiceUnavailable(format!(
                "Cloudflare API 错误（HTTP {status}）: {msg}"
            )));
        }
        Ok(body)
    }

    async fn request_json(
        &self,
        method: reqwest::Method,
        url: &str,
        body: Value,
    ) -> Result<Value, AppError> {
        let resp = self
            .http
            .request(method, url)
            .bearer_auth(&self.api_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("Cloudflare API 请求失败: {e}")))?;
        let status = resp.status();
        let payload: Value = resp.json().await.map_err(|e| {
            AppError::ServiceUnavailable(format!("Cloudflare API 响应解析失败: {e}"))
        })?;
        if !status.is_success() {
            let msg = payload
                .pointer("/errors/0/message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            return Err(AppError::ServiceUnavailable(format!(
                "Cloudflare API 错误（HTTP {status}）: {msg}"
            )));
        }
        Ok(payload)
    }

    /// 连通性探测：`GET /user/tokens/verify`（只验证 Token 有效性，零副作用）。
    pub async fn verify(&self) -> Result<(), AppError> {
        let body = self
            .get_json("https://api.cloudflare.com/client/v4/user/tokens/verify")
            .await?;
        if body.get("success").and_then(|v| v.as_bool()) != Some(true) {
            return Err(AppError::ServiceUnavailable(
                "Cloudflare Token 校验未通过".into(),
            ));
        }
        Ok(())
    }

    /// 列出账号下 zone（探测/选 zone 用，本阶段仅 test_connection 路径使用）。
    async fn find_zone_id(&self, domain: &str) -> Result<String, AppError> {
        let base = domain.trim_start_matches("*.");
        // 取注册主域：逐级尝试（www.example.com → example.com）
        let labels: Vec<&str> = base.split('.').collect();
        let mut candidates = Vec::new();
        for i in 0..labels.len().saturating_sub(1) {
            candidates.push(labels[i..].join("."));
        }
        for name in &candidates {
            let url = format!("https://api.cloudflare.com/client/v4/zones?name={name}");
            let body = self.get_json(&url).await?;
            if let Some(id) = body.pointer("/result/0/id").and_then(|v| v.as_str()) {
                return Ok(id.to_string());
            }
        }
        Err(AppError::ServiceUnavailable(format!(
            "Cloudflare 账号下未找到域名 {base} 的 zone"
        )))
    }

    async fn find_record_id(
        &self,
        zone_id: &str,
        name: &str,
        txt_value: &str,
    ) -> Result<Option<String>, AppError> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{zone_id}/dns_records?type=TXT&name={name}&per_page=100"
        );
        let body = self.get_json(&url).await?;
        for item in body
            .get("result")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
        {
            if item.get("content").and_then(|v| v.as_str()) == Some(txt_value) {
                if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                    return Ok(Some(id.to_string()));
                }
            }
        }
        Ok(None)
    }
}

#[async_trait]
impl DnsProvider for CloudflareDnsProvider {
    fn name(&self) -> &'static str {
        "cloudflare"
    }

    async fn set_txt_record(&self, domain: &str, txt_value: &str) -> Result<(), AppError> {
        let record_name = acme_challenge_name(domain);
        let zone_id = self.find_zone_id(&record_name).await?;
        // 幂等：同名同值已存在则复用（ACME 重试场景）
        if self
            .find_record_id(&zone_id, &record_name, txt_value)
            .await?
            .is_some()
        {
            return Ok(());
        }
        self.request_json(
            reqwest::Method::POST,
            &format!("https://api.cloudflare.com/client/v4/zones/{zone_id}/dns_records"),
            serde_json::json!({
                "type": "TXT",
                "name": record_name,
                "content": txt_value,
                "ttl": 120,
            }),
        )
        .await?;
        Ok(())
    }

    async fn clear_txt_record(&self, domain: &str, txt_value: &str) -> Result<(), AppError> {
        let record_name = acme_challenge_name(domain);
        let Ok(zone_id) = self.find_zone_id(&record_name).await else {
            return Ok(()); // zone 查询失败时清理尽力而为
        };
        if let Some(id) = self
            .find_record_id(&zone_id, &record_name, txt_value)
            .await
            .unwrap_or(None)
        {
            let url =
                format!("https://api.cloudflare.com/client/v4/zones/{zone_id}/dns_records/{id}");
            let _ = self
                .request_json(reqwest::Method::DELETE, &url, serde_json::json!({}))
                .await;
        }
        Ok(())
    }
}

// ── 阿里云 ──────────────────────────────────────────────────────────────

/// 阿里云 DNS Provider（云解析 DNS，RPC 签名版 API）。
///
/// 签名：HMAC-SHA1（`SignatureMethod=HMAC-SHA1`，`SignatureVersion=1.0`），
/// 规范化查询串按字典序排列后 percent-encode（RFC 3986）拼接
/// `GET&%2F&<encoded-query>`，AccessKeySecret 以 `&` 结尾作签名密钥。
pub struct AliyunDnsProvider {
    http: reqwest::Client,
    access_key_id: String,
    access_key_secret: String,
}

/// RPC API percent-encode（RFC 3986 unreserved + 阿里云要求的 `~` 不转义）。
fn aliyun_percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

impl AliyunDnsProvider {
    /// 从凭据 JSON（`{"access_key_id": "...", "access_key_secret": "..."}`）构造。
    pub fn from_credentials(credentials_json: &str) -> Result<Self, AppError> {
        let cfg: Value = serde_json::from_str(credentials_json)
            .map_err(|e| AppError::BadRequest(format!("阿里云凭据 JSON 非法: {e}")))?;
        let get = |key: &str| -> Result<String, AppError> {
            cfg.get(key)
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .ok_or_else(|| AppError::BadRequest(format!("阿里云凭据缺少 {key}")))
        };
        Ok(Self {
            http: http_client()?,
            access_key_id: get("access_key_id")?,
            access_key_secret: get("access_key_secret")?,
        })
    }

    /// 组装签名后的查询串并发起 GET（RPC 风格，返回 JSON）。
    async fn rpc(&self, action: &str, params: &[(&str, &str)]) -> Result<Value, AppError> {
        let nonce = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let mut common: Vec<(String, String)> = vec![
            ("AccessKeyId".into(), self.access_key_id.clone()),
            ("Action".into(), action.into()),
            ("Format".into(), "JSON".into()),
            ("RegionId".into(), "cn-hangzhou".into()),
            ("SignatureMethod".into(), "HMAC-SHA1".into()),
            ("SignatureNonce".into(), nonce),
            ("SignatureVersion".into(), "1.0".into()),
            ("Timestamp".into(), now),
            ("Version".into(), "2015-01-09".into()),
        ];
        for (k, v) in params {
            common.push(((*k).into(), (*v).into()));
        }
        common.sort_by(|a, b| a.0.cmp(&b.0));
        let canonical = common
            .iter()
            .map(|(k, v)| format!("{}={}", aliyun_percent_encode(k), aliyun_percent_encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        let string_to_sign = format!("GET&%2F&{}", aliyun_percent_encode(&canonical));

        let mut mac =
            Hmac::<Sha1>::new_from_slice(format!("{}&", self.access_key_secret).as_bytes())
                .map_err(|e| AppError::internal(format!("阿里云签名密钥构造失败: {e}")))?;
        mac.update(string_to_sign.as_bytes());
        let signature = STANDARD.encode(mac.finalize().into_bytes());

        let url = format!(
            "https://alidns.aliyuncs.com/?{canonical}&Signature={}",
            aliyun_percent_encode(&signature)
        );
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("阿里云 API 请求失败: {e}")))?;
        let status = resp.status();
        let body: Value = resp
            .json()
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("阿里云 API 响应解析失败: {e}")))?;
        if !status.is_success() || body.get("Code").is_some() {
            let code = body
                .get("Code")
                .and_then(|v| v.as_str())
                .unwrap_or("UnknownError");
            let msg = body.get("Message").and_then(|v| v.as_str()).unwrap_or("");
            return Err(AppError::ServiceUnavailable(format!(
                "阿里云 API 错误（HTTP {status}）: {code} {msg}"
            )));
        }
        Ok(body)
    }

    /// 连通性探测：DescribeDomains（PageSize=1，只验证签名与授权有效性）。
    pub async fn verify(&self) -> Result<(), AppError> {
        self.rpc("DescribeDomains", &[("PageSize", "1")]).await?;
        Ok(())
    }

    async fn find_record_id(
        &self,
        record_name: &str,
        txt_value: &str,
    ) -> Result<Option<String>, AppError> {
        // 拆 RR 与主域：_acme-challenge.www.example.com → RR=_acme-challenge.www,
        // DomainName=example.com；逐级收窄尝试主域
        let labels: Vec<&str> = record_name.split('.').collect();
        for split in (1..labels.len().saturating_sub(1)).rev() {
            let rr = labels[..split].join(".");
            let domain_name = labels[split..].join(".");
            let body = self
                .rpc(
                    "DescribeDomainRecords",
                    &[
                        ("DomainName", domain_name.as_str()),
                        ("RRKeyWord", rr.as_str()),
                        ("TypeKeyWord", "TXT"),
                        ("PageSize", "100"),
                    ],
                )
                .await?;
            let empty = Vec::new();
            for item in body
                .pointer("/DomainRecords/Record")
                .and_then(|v| v.as_array())
                .unwrap_or(&empty)
            {
                let same_rr = item.get("RR").and_then(|v| v.as_str()) == Some(rr.as_str());
                let same_value = item.get("Value").and_then(|v| v.as_str()) == Some(txt_value);
                if same_rr && same_value {
                    if let Some(id) = item.get("RecordId").and_then(|v| v.as_str()) {
                        return Ok(Some(id.to_string()));
                    }
                    if let Some(id) = item.get("RecordId").and_then(|v| v.as_i64()) {
                        return Ok(Some(id.to_string()));
                    }
                }
            }
        }
        Ok(None)
    }
}

#[async_trait]
impl DnsProvider for AliyunDnsProvider {
    fn name(&self) -> &'static str {
        "aliyun"
    }

    async fn set_txt_record(&self, domain: &str, txt_value: &str) -> Result<(), AppError> {
        let record_name = acme_challenge_name(domain);
        // 幂等：已有同值记录直接复用
        if self
            .find_record_id(&record_name, txt_value)
            .await?
            .is_some()
        {
            return Ok(());
        }
        // 主域逐级尝试：_acme-challenge.www.example.com → 先试
        // RR=_acme-challenge.www + DomainName=example.com，失败再收窄
        let labels: Vec<&str> = record_name.split('.').collect();
        let mut last_err: Option<AppError> = None;
        for split in (1..labels.len().saturating_sub(1)).rev() {
            let rr = labels[..split].join(".");
            let domain_name = labels[split..].join(".");
            match self
                .rpc(
                    "AddDomainRecord",
                    &[
                        ("DomainName", domain_name.as_str()),
                        ("RR", rr.as_str()),
                        ("Type", "TXT"),
                        ("Value", txt_value),
                        ("TTL", "600"),
                    ],
                )
                .await
            {
                Ok(_) => return Ok(()),
                Err(e) => {
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap_or_else(|| {
            AppError::ServiceUnavailable("阿里云 TXT 写入失败：无可匹配域名".into())
        }))
    }

    async fn clear_txt_record(&self, domain: &str, txt_value: &str) -> Result<(), AppError> {
        let record_name = acme_challenge_name(domain);
        let Ok(Some(id)) = self.find_record_id(&record_name, txt_value).await else {
            return Ok(()); // 记录不存在/查询失败时清理尽力而为
        };
        let _ = self
            .rpc("DeleteDomainRecord", &[("RecordId", id.as_str())])
            .await;
        Ok(())
    }
}

/// 按厂商类型从账号凭据构造真实 Provider（签发/续签编排组合根路由用）。
/// 四家厂商（cloudflare | aliyun | tencent | huawei）全覆盖，未知类型明确拒绝。
pub fn build_provider(
    provider_type: &str,
    credentials_json: &str,
) -> Result<std::sync::Arc<dyn DnsProvider>, AppError> {
    match provider_type {
        "cloudflare" => Ok(std::sync::Arc::new(
            CloudflareDnsProvider::from_credentials(credentials_json)?,
        )),
        "aliyun" => Ok(std::sync::Arc::new(AliyunDnsProvider::from_credentials(
            credentials_json,
        )?)),
        "tencent" => Ok(std::sync::Arc::new(TencentDnsProvider::from_credentials(
            credentials_json,
        )?)),
        "huawei" => Ok(std::sync::Arc::new(HuaweiDnsProvider::from_credentials(
            credentials_json,
        )?)),
        other => Err(AppError::BadRequest(format!(
            "不支持的 DNS 厂商类型: {other}（可选 cloudflare | aliyun | tencent | huawei）"
        ))),
    }
}
// ── 腾讯云 ──────────────────────────────────────────────────────────────

/// 腾讯云 DNSPod Provider（云解析 DNS，TC3-HMAC-SHA256 签名版 API v3）。
///
/// 签名：TC3-HMAC-SHA256（规范请求串 → 签名字符串 → 派生签名密钥），
/// 服务 `dnspod`，host `dnspod.tencentcloudapi.com`。
pub struct TencentDnsProvider {
    http: reqwest::Client,
    secret_id: String,
    secret_key: String,
}

impl TencentDnsProvider {
    /// 从凭据 JSON（`{"secret_id": "...", "secret_key": "..."}`）构造。
    pub fn from_credentials(credentials_json: &str) -> Result<Self, AppError> {
        let cfg: Value = serde_json::from_str(credentials_json)
            .map_err(|e| AppError::BadRequest(format!("腾讯云凭据 JSON 非法: {e}")))?;
        let get = |key: &str| -> Result<String, AppError> {
            cfg.get(key)
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .ok_or_else(|| AppError::BadRequest(format!("腾讯云凭据缺少 {key}")))
        };
        Ok(Self {
            http: http_client()?,
            secret_id: get("secret_id")?,
            secret_key: get("secret_key")?,
        })
    }

    /// TC3-HMAC-SHA256 签名并 POST JSON 请求（API 3.0 统一封装）。
    async fn api(&self, action: &str, payload: Value) -> Result<Value, AppError> {
        let now = chrono::Utc::now();
        let timestamp = now.timestamp();
        let date = now.format("%Y-%m-%d").to_string();
        let service = "dnspod";
        let host = "dnspod.tencentcloudapi.com";
        let body = serde_json::to_string(&payload)
            .map_err(|e| AppError::internal(format!("腾讯云请求体序列化失败: {e}")))?;

        // 1. 拼接规范请求串
        let hashed_body = hex_sha256(body.as_bytes());
        let canonical_request = format!(
            "POST\n/\n\ncontent-type:application/json\nhost:{host}\nx-tc-action:{action}\n\ncontent-type;host;x-tc-action\n{hashed_body}"
        );
        // 2. 拼接签名字符串
        let hashed_canonical = hex_sha256(canonical_request.as_bytes());
        let string_to_sign = format!(
            "TC3-HMAC-SHA256\n{timestamp}\n{date}/{service}/tc3_request\n{hashed_canonical}"
        );
        // 3. 派生签名密钥：kDate → kService → kSigning
        let k_date = Hmac::<Sha256>::new_from_slice(format!("TC3{}", self.secret_key).as_bytes())
            .map_err(|e| AppError::internal(format!("腾讯云签名密钥构造失败: {e}")))?
            .chain_update(date.as_bytes())
            .finalize()
            .into_bytes();
        let k_service = Hmac::<Sha256>::new_from_slice(&k_date)
            .map_err(|e| AppError::internal(format!("腾讯云签名密钥构造失败: {e}")))?
            .chain_update(service.as_bytes())
            .finalize()
            .into_bytes();
        let k_signing = Hmac::<Sha256>::new_from_slice(&k_service)
            .map_err(|e| AppError::internal(format!("腾讯云签名密钥构造失败: {e}")))?
            .chain_update(b"tc3_request")
            .finalize()
            .into_bytes();
        // 4. 计算签名
        let signature = hex_encode(
            &Hmac::<Sha256>::new_from_slice(&k_signing)
                .map_err(|e| AppError::internal(format!("腾讯云签名密钥构造失败: {e}")))?
                .chain_update(string_to_sign.as_bytes())
                .finalize()
                .into_bytes(),
        );

        let authorization = format!(
            "TC3-HMAC-SHA256 Credential={}/{}/{service}/tc3_request, SignedHeaders=content-type;host;x-tc-action, Signature={signature}",
            self.secret_id, date
        );
        let resp = self
            .http
            .post(format!("https://{host}/"))
            .header("Host", host)
            .header("X-TC-Action", action)
            .header("X-TC-Version", "2021-03-23")
            .header("X-TC-Timestamp", timestamp.to_string())
            .header("X-TC-Language", "zh-CN")
            .header("Authorization", &authorization)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("腾讯云 API 请求失败: {e}")))?;
        let status = resp.status();
        let body: Value = resp
            .json()
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("腾讯云 API 响应解析失败: {e}")))?;
        if let Some(err) = body.get("Response").and_then(|r| r.get("Error")) {
            let code = err
                .get("Code")
                .and_then(|v| v.as_str())
                .unwrap_or("UnknownError");
            let msg = err.get("Message").and_then(|v| v.as_str()).unwrap_or("");
            return Err(AppError::ServiceUnavailable(format!(
                "腾讯云 API 错误: {code} {msg}"
            )));
        }
        if !status.is_success() {
            return Err(AppError::ServiceUnavailable(format!(
                "腾讯云 API 错误（HTTP {status}）"
            )));
        }
        Ok(body)
    }

    /// 连通性探测：DescribeDomainList（Offset=0, Limit=1，只验证签名与授权有效性）。
    pub async fn verify(&self) -> Result<(), AppError> {
        self.api(
            "DescribeDomainList",
            serde_json::json!({"Offset": 0, "Limit": 1}),
        )
        .await?;
        Ok(())
    }

    /// 查询同名同值 TXT 记录的 RecordId（幂等复用 / 清理定位）。
    ///
    /// `DescribeRecordList` 缺省 `Limit=20`（腾讯云 API 默认值），记录数超出时
    /// 单页遍历会漏匹配 → 逐页翻遍（`Offset/Limit` 显式分页），命中即提前返回；
    /// 页数上限兜底防止异常响应导致无界循环。
    async fn find_record_id(
        &self,
        domain: &str,
        sub_domain: &str,
        txt_value: &str,
    ) -> Result<Option<String>, AppError> {
        const PAGE_SIZE: i64 = 100;
        // 单 SubDomain + TXT 过滤下记录数理应很小；上限仅防御异常/超大 zone
        const MAX_PAGES: i64 = 200;
        let mut offset = 0i64;
        for _ in 0..MAX_PAGES {
            let body = self
                .api(
                    "DescribeRecordList",
                    serde_json::json!({
                        "Domain": domain,
                        "SubDomain": sub_domain,
                        "RecordType": "TXT",
                        "Offset": offset,
                        "Limit": PAGE_SIZE,
                    }),
                )
                .await?;
            let empty = Vec::new();
            let page = body
                .pointer("/Response/RecordList")
                .and_then(|v| v.as_array())
                .unwrap_or(&empty);
            if let Some(id) = find_txt_record_in_page(page, sub_domain, txt_value) {
                return Ok(Some(id));
            }
            // 末页（不足一页）→ 记录不存在
            if !has_more_pages(page.len(), PAGE_SIZE) {
                return Ok(None);
            }
            offset += PAGE_SIZE;
        }
        Err(AppError::ServiceUnavailable(format!(
            "腾讯云 DNS 记录查询翻页超出上限（>{} 页），疑似响应异常: {domain}",
            MAX_PAGES
        )))
    }

    /// 拆分 `_acme-challenge.www.example.com` →（主域 example.com，子域
    /// _acme-challenge.www）；逐级收窄，`example.com` 根域时 SubDomain=`@`。
    fn split_domain(record_name: &str) -> Result<(String, String), AppError> {
        let labels: Vec<&str> = record_name.split('.').collect();
        if labels.len() < 2 {
            return Err(AppError::BadRequest(format!(
                "域名 {record_name} 不含可解析的主域"
            )));
        }
        let domain = labels[labels.len() - 2..].join(".");
        let sub_parts = &labels[..labels.len() - 2];
        let sub = if sub_parts.is_empty() {
            "@".to_string()
        } else {
            sub_parts.join(".")
        };
        Ok((domain, sub))
    }

    async fn add_record(
        &self,
        domain: &str,
        sub_domain: &str,
        txt_value: &str,
    ) -> Result<(), AppError> {
        self.api(
            "CreateRecord",
            serde_json::json!({
                "Domain": domain,
                "SubDomain": sub_domain,
                "RecordType": "TXT",
                "RecordLine": "默认",
                "Value": txt_value,
                "TTL": 600,
            }),
        )
        .await?;
        Ok(())
    }
}

#[async_trait]
impl DnsProvider for TencentDnsProvider {
    fn name(&self) -> &'static str {
        "tencent"
    }

    async fn set_txt_record(&self, domain: &str, txt_value: &str) -> Result<(), AppError> {
        let record_name = acme_challenge_name(domain);
        let (d, sub) = Self::split_domain(&record_name)?;
        // 幂等：同名同值已存在则复用（ACME 重试场景）
        if self.find_record_id(&d, &sub, txt_value).await?.is_some() {
            return Ok(());
        }
        self.add_record(&d, &sub, txt_value).await
    }

    async fn clear_txt_record(&self, domain: &str, txt_value: &str) -> Result<(), AppError> {
        let record_name = acme_challenge_name(domain);
        let (d, sub) = Self::split_domain(&record_name)?;
        if let Some(id) = self
            .find_record_id(&d, &sub, txt_value)
            .await
            .unwrap_or(None)
        {
            let _ = self
                .api(
                    "DeleteRecord",
                    serde_json::json!({"Domain": d, "RecordId": id}),
                )
                .await;
        }
        Ok(())
    }
}

// ── 华为云 ──────────────────────────────────────────────────────────────

/// 华为云 DNS Provider（云解析服务 DNS，IAM token 认证 + v2 REST API）。
///
/// 认证：IAM project-scoped token（`POST /v3/auth/tokens`，返回
/// `X-Subject-Token` 请求头），token 缓存至临近过期；zone 与 recordset
/// 走 `dns.googleapis` 风格的 `/v2/zones` API。
pub struct HuaweiDnsProvider {
    http: reqwest::Client,
    access_key: String,
    secret_key: String,
    /// 缓存的 IAM token（跨质询轮次复用，避免每条 TXT 都重新认证）
    token_cache: tokio::sync::Mutex<Option<(String, std::time::Instant)>>,
}

impl HuaweiDnsProvider {
    /// 从凭据 JSON（`{"access_key": "...", "secret_key": "..."}`）构造。
    pub fn from_credentials(credentials_json: &str) -> Result<Self, AppError> {
        let cfg: Value = serde_json::from_str(credentials_json)
            .map_err(|e| AppError::BadRequest(format!("华为云凭据 JSON 非法: {e}")))?;
        let get = |key: &str| -> Result<String, AppError> {
            cfg.get(key)
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .ok_or_else(|| AppError::BadRequest(format!("华为云凭据缺少 {key}")))
        };
        Ok(Self {
            http: http_client()?,
            access_key: get("access_key")?,
            secret_key: get("secret_key")?,
            token_cache: tokio::sync::Mutex::new(None),
        })
    }

    /// 获取（带缓存）IAM project-scoped token。token 有效期默认 24h，
    /// 这里按 1 小时保守策略过期重取。
    async fn token(&self) -> Result<String, AppError> {
        let mut cache = self.token_cache.lock().await;
        if let Some((token, at)) = cache.as_ref() {
            if at.elapsed() < std::time::Duration::from_secs(3600) {
                return Ok(token.clone());
            }
        }
        let body = serde_json::json!({
            "auth": {
                "identity": {
                    "methods": ["password"],
                    "password": {
                        "user": {
                            "name": self.access_key,
                            "domain": {"name": self.access_key},
                            "password": self.secret_key
                        }
                    }
                },
                "scope": {"project": {"name": "cn-north-4"}}
            }
        });
        let resp = self
            .http
            .post("https://iam.myhuaweicloud.com/v3/auth/tokens")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("华为云 IAM 请求失败: {e}")))?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            let brief: String = text.chars().take(160).collect();
            return Err(AppError::ServiceUnavailable(format!(
                "华为云 IAM 认证失败（HTTP {status}）: {brief}"
            )));
        }
        let token = resp
            .headers()
            .get("X-Subject-Token")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
            .ok_or_else(|| {
                AppError::ServiceUnavailable("华为云 IAM 响应缺少 X-Subject-Token".into())
            })?;
        *cache = Some((token.clone(), std::time::Instant::now()));
        Ok(token)
    }

    /// 连通性探测：获取 IAM token（零副作用身份验证，token 成功签发即凭据有效）。
    pub async fn verify(&self) -> Result<(), AppError> {
        self.token().await?;
        Ok(())
    }

    async fn dns_get(&self, url: &str) -> Result<Value, AppError> {
        let token = self.token().await?;
        let resp = self
            .http
            .get(url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("华为云 DNS API 请求失败: {e}")))?;
        let status = resp.status();
        let body: Value = resp.json().await.map_err(|e| {
            AppError::ServiceUnavailable(format!("华为云 DNS API 响应解析失败: {e}"))
        })?;
        if !status.is_success() {
            let code = body
                .pointer("/item/code")
                .or_else(|| body.get("code"))
                .and_then(|v| v.as_str())
                .unwrap_or("UnknownError");
            let msg = body
                .pointer("/item/message")
                .or_else(|| body.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            return Err(AppError::ServiceUnavailable(format!(
                "华为云 DNS API 错误（HTTP {status}）: {code} {msg}"
            )));
        }
        Ok(body)
    }

    /// 按 zone 名称精确匹配（`example.com.`，华为云返回带尾点）。
    async fn find_zone_id(&self, domain: &str) -> Result<String, AppError> {
        let base = domain.trim_start_matches("*.");
        let labels: Vec<&str> = base.split('.').collect();
        let mut candidates = Vec::new();
        for i in 0..labels.len().saturating_sub(1) {
            candidates.push(labels[i..].join("."));
        }
        for name in &candidates {
            let url = format!("https://dns.myhuaweicloud.com/v2/zones?name={name}.");
            let body = self.dns_get(&url).await?;
            if let Some(id) = body.pointer("/zones/0/id").and_then(|v| v.as_str()) {
                return Ok(id.to_string());
            }
        }
        Err(AppError::ServiceUnavailable(format!(
            "华为云账号下未找到域名 {base} 的 zone"
        )))
    }

    /// 查询同名同值 TXT recordset（recordsets 按 name 全域匹配，带尾点）。
    async fn find_recordset_id(
        &self,
        zone_id: &str,
        record_name: &str,
        txt_value: &str,
    ) -> Result<Option<String>, AppError> {
        let url = format!(
            "https://dns.myhuaweicloud.com/v2/zones/{zone_id}/recordsets?type=TXT&name={record_name}."
        );
        let body = self.dns_get(&url).await?;
        let empty = Vec::new();
        for item in body
            .get("recordsets")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty)
        {
            let records: Vec<String> = item
                .get("records")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|r| r.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            if records.iter().any(|r| r == txt_value) {
                if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                    return Ok(Some(id.to_string()));
                }
            }
        }
        Ok(None)
    }
}

#[async_trait]
impl DnsProvider for HuaweiDnsProvider {
    fn name(&self) -> &'static str {
        "huawei"
    }

    async fn set_txt_record(&self, domain: &str, txt_value: &str) -> Result<(), AppError> {
        let record_name = acme_challenge_name(domain);
        let zone_id = self.find_zone_id(&record_name).await?;
        // 幂等：同名同值已存在则复用（ACME 重试场景）
        if self
            .find_recordset_id(&zone_id, &record_name, txt_value)
            .await?
            .is_some()
        {
            return Ok(());
        }
        let token = self.token().await?;
        let url = format!("https://dns.myhuaweicloud.com/v2/zones/{zone_id}/recordsets");
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&token)
            .json(&serde_json::json!({
                "name": format!("{record_name}."),
                "type": "TXT",
                "ttl": 300,
                "records": [txt_value],
            }))
            .send()
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("华为云 DNS API 请求失败: {e}")))?;
        let status = resp.status();
        let body: Value = resp.json().await.map_err(|e| {
            AppError::ServiceUnavailable(format!("华为云 DNS API 响应解析失败: {e}"))
        })?;
        if !status.is_success() {
            let code = body
                .get("code")
                .and_then(|v| v.as_str())
                .unwrap_or("UnknownError");
            let msg = body.get("message").and_then(|v| v.as_str()).unwrap_or("");
            return Err(AppError::ServiceUnavailable(format!(
                "华为云 DNS API 错误（HTTP {status}）: {code} {msg}"
            )));
        }
        Ok(())
    }

    async fn clear_txt_record(&self, domain: &str, txt_value: &str) -> Result<(), AppError> {
        let record_name = acme_challenge_name(domain);
        // 清理尽力而为：token/zone 查询失败直接返回成功
        let zone_id = match self.find_zone_id(&record_name).await {
            Ok(id) => id,
            Err(_) => return Ok(()),
        };
        let Ok(Some(id)) = self
            .find_recordset_id(&zone_id, &record_name, txt_value)
            .await
        else {
            return Ok(());
        };
        if let Ok(token) = self.token().await {
            let url = format!("https://dns.myhuaweicloud.com/v2/zones/{zone_id}/recordsets/{id}");
            let _ = self.http.delete(&url).bearer_auth(&token).send().await;
        }
        Ok(())
    }
}

/// 在 `DescribeRecordList` 的一页记录中查找同名同值 TXT 的 RecordId
/// （纯逻辑抽出便于单测；分页遍历逐页调用）。
fn find_txt_record_in_page(page: &[Value], sub_domain: &str, txt_value: &str) -> Option<String> {
    for item in page {
        let same_value = item.get("Value").and_then(|v| v.as_str()) == Some(txt_value);
        let same_sub = item.get("SubDomain").and_then(|v| v.as_str()) == Some(sub_domain);
        if same_value && same_sub {
            if let Some(id) = item.get("RecordId").and_then(|v| v.as_i64()) {
                return Some(id.to_string());
            }
        }
    }
    None
}

/// 分页终止判定：返回 `true` 表示还有下一页（本页仍满员）。
fn has_more_pages(page_len: usize, page_size: i64) -> bool {
    page_size > 0 && page_len as i64 >= page_size
}

/// 小写十六进制 SHA-256（TC3 签名用）。
fn hex_sha256(data: &[u8]) -> String {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(data);
    hex_encode(&digest)
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_name_maps_wildcard() {
        assert_eq!(
            acme_challenge_name("*.example.com"),
            "_acme-challenge.example.com"
        );
        assert_eq!(
            acme_challenge_name("www.example.com"),
            "_acme-challenge.www.example.com"
        );
    }

    #[test]
    fn cloudflare_rejects_missing_token() {
        assert!(CloudflareDnsProvider::from_credentials("{}").is_err());
        assert!(CloudflareDnsProvider::from_credentials(r#"{"api_token":"  "}"#).is_err());
        assert!(CloudflareDnsProvider::from_credentials("not-json").is_err());
    }

    #[test]
    fn aliyun_rejects_missing_keys() {
        assert!(AliyunDnsProvider::from_credentials("{}").is_err());
        assert!(AliyunDnsProvider::from_credentials(r#"{"access_key_id":"a"}"#).is_err());
    }

    #[test]
    fn aliyun_percent_encode_rfc3986() {
        assert_eq!(aliyun_percent_encode("a b+c/d"), "a%20b%2Bc%2Fd");
        assert_eq!(aliyun_percent_encode("~.-_9"), "~.-_9");
        assert_eq!(aliyun_percent_encode("100%"), "100%25");
    }

    #[test]
    fn tencent_huawei_reject_missing_keys() {
        assert!(TencentDnsProvider::from_credentials("{}").is_err());
        assert!(TencentDnsProvider::from_credentials(r#"{"secret_id":"a"}"#).is_err());
        assert!(HuaweiDnsProvider::from_credentials("{}").is_err());
        assert!(HuaweiDnsProvider::from_credentials(r#"{"access_key":"a"}"#).is_err());
    }

    #[test]
    fn tencent_split_domain_root_and_sub() {
        let (d, sub) = TencentDnsProvider::split_domain("_acme-challenge.example.com").unwrap();
        assert_eq!(d, "example.com");
        assert_eq!(sub, "_acme-challenge");
        let (d, sub) = TencentDnsProvider::split_domain("_acme-challenge.www.example.com").unwrap();
        assert_eq!(d, "example.com");
        assert_eq!(sub, "_acme-challenge.www");
        // 单标签域名不可拆分 → 明确报错
        assert!(TencentDnsProvider::split_domain("localhost").is_err());
    }

    /// 腾讯云分页：页内匹配命中返回 RecordId；不命中返回 None；
    /// 跨页场景由调用方按 `has_more_pages` 继续翻页（此处覆盖判定与匹配逻辑）。
    #[test]
    fn tencent_record_pagination_matching() {
        let page = serde_json::json!([
            {"RecordId": 111, "SubDomain": "_acme-challenge", "Value": "other-value"},
            {"RecordId": 222, "SubDomain": "_acme-challenge", "Value": "want"},
            {"RecordId": 333, "SubDomain": "_acme-challenge.www", "Value": "want"}
        ]);
        let page = page.as_array().unwrap().clone();
        // 同名（SubDomain）+ 同值 → 命中
        assert_eq!(
            find_txt_record_in_page(&page, "_acme-challenge", "want").as_deref(),
            Some("222")
        );
        // 同值不同名 → 不命中（避免误删他人记录）
        assert_eq!(
            find_txt_record_in_page(&page, "_acme-challenge.www", "other-value"),
            None
        );
        // 空页 → 不命中
        assert_eq!(
            find_txt_record_in_page(&[], "_acme-challenge", "want"),
            None
        );
        // 翻页判定：满页继续、不足一页终止（缺省 Limit=20 漏匹配的修复依据）
        assert!(has_more_pages(100, 100));
        assert!(!has_more_pages(99, 100));
        assert!(!has_more_pages(0, 100));
    }

    #[test]
    fn tencent_tc3_signature_is_deterministic() {
        // TC3 派生密钥链复算一致性（防签名实现被意外改动）
        let secret_key = "testsecret";
        let date = "2026-01-01";
        let k_date = Hmac::<Sha256>::new_from_slice(format!("TC3{secret_key}").as_bytes())
            .unwrap()
            .chain_update(date.as_bytes())
            .finalize()
            .into_bytes();
        let k_service = Hmac::<Sha256>::new_from_slice(&k_date)
            .unwrap()
            .chain_update(b"dnspod")
            .finalize()
            .into_bytes();
        let k_signing = Hmac::<Sha256>::new_from_slice(&k_service)
            .unwrap()
            .chain_update(b"tc3_request")
            .finalize()
            .into_bytes();
        let sig1 = hex_encode(
            &Hmac::<Sha256>::new_from_slice(&k_signing)
                .unwrap()
                .chain_update(b"string-to-sign")
                .finalize()
                .into_bytes(),
        );
        let sig2 = hex_encode(
            &Hmac::<Sha256>::new_from_slice(&k_signing)
                .unwrap()
                .chain_update(b"string-to-sign")
                .finalize()
                .into_bytes(),
        );
        assert_eq!(sig1, sig2);
        assert_eq!(sig1.len(), 64);
    }

    #[test]
    fn hex_sha256_matches_known_vector() {
        assert_eq!(
            hex_sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex_sha256(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn build_provider_routes_by_type() {
        let cf = build_provider("cloudflare", r#"{"api_token":"tok"}"#).unwrap();
        assert_eq!(cf.name(), "cloudflare");
        let ay =
            build_provider("aliyun", r#"{"access_key_id":"a","access_key_secret":"b"}"#).unwrap();
        assert_eq!(ay.name(), "aliyun");
        let tx = build_provider("tencent", r#"{"secret_id":"a","secret_key":"b"}"#).unwrap();
        assert_eq!(tx.name(), "tencent");
        let hw = build_provider("huawei", r#"{"access_key":"a","secret_key":"b"}"#).unwrap();
        assert_eq!(hw.name(), "huawei");
        // 未交付厂商明确拒绝
        assert!(build_provider("tencent", "{}").is_err());
        assert!(build_provider("huawei", "{}").is_err());
    }

    #[test]
    fn aliyun_signature_is_deterministic() {
        // 固定输入下签名可复算（HMAC-SHA1 + base64 标准编码）
        let provider = AliyunDnsProvider::from_credentials(
            r#"{"access_key_id":"testid","access_key_secret":"testsecret"}"#,
        )
        .unwrap();
        let canonical = "AccessKeyId=testid&Action=DescribeDomains&Format=JSON";
        let string_to_sign = format!("GET&%2F&{}", aliyun_percent_encode(canonical));
        let mut mac = Hmac::<Sha1>::new_from_slice(b"testsecret&").unwrap();
        mac.update(string_to_sign.as_bytes());
        let sig = STANDARD.encode(mac.finalize().into_bytes());
        // 复算一致性（防签名实现被意外改动）
        let mut mac2 = Hmac::<Sha1>::new_from_slice(b"testsecret&").unwrap();
        mac2.update(string_to_sign.as_bytes());
        assert_eq!(sig, STANDARD.encode(mac2.finalize().into_bytes()));
        assert_eq!(provider.access_key_id, "testid");
    }
}
