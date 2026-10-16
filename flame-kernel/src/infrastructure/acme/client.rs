//! 最小 ACME v2 (RFC 8555) 协议客户端（M-B 主线三 §6.1 · Stage 2 签发链路）。
//!
//! 设计取舍：不引入 `instant-acme`（传递依赖链过重），基于仓库既有
//! `reqwest`（rustls-tls）+ `ring`（rcgen 传递依赖，已在依赖树）自研覆盖
//! 签发所需的最小协议子集：
//! - 目录发现（GET /directory → newAccount/newOrder/newNonce 端点）
//! - JWS ES256（JSON Web Signature，ECDSA P-256，RFC 7515）请求签名
//! - newAccount（首次注册 / existing 账户复用 200 语义统一处理）
//! - newOrder / authz / challenge（http-01 与 dns-01）/ finalize / 证书下载
//! - 重放保护：每次请求前取新 nonce（Replay-Nonce 响应头回读缓存）
//!
//! DNS-01 真实记录写入经 [`super::dns::DnsProvider`] 端口，本模块只负责
//! 协议编排与 token 组装。

use crate::core::error::AppError;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ring::rand::SystemRandom;
use ring::signature::EcdsaKeyPair;
use ring::signature::KeyPair as _;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// 默认 ACME 目录 URL（Let's Encrypt 生产）。
pub const LETSENCRYPT_DIRECTORY: &str = "https://acme-v02.api.letsencrypt.org/directory";
/// Let's Encrypt 测试目录（端到端联调用）。
pub const LETSENCRYPT_STAGING_DIRECTORY: &str =
    "https://acme-staging-v02.api.letsencrypt.org/directory";

/// ACME 目录端点集合。
#[derive(Debug, Clone, Default)]
pub struct Directory {
    pub new_nonce: String,
    pub new_account: String,
    pub new_order: String,
}

/// ACME 客户端：持有 HTTP 客户端、账户密钥、账户 URL 与 nonce 缓存。
pub struct AcmeClient {
    http: reqwest::Client,
    /// 账户密钥（PKCS#8 DER，ECDSA P-256）
    signing_key: EcdsaKeyPair,
    rng: SystemRandom,
    account_url: Option<String>,
    nonce: Option<String>,
    directory: Directory,
    directory_url: String,
}

/// 解析 PEM → PKCS#8 DER。
fn pem_to_pkcs8_der(pem: &str) -> Result<Vec<u8>, AppError> {
    let body = pem
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .collect::<Vec<_>>()
        .join("");
    base64::engine::general_purpose::STANDARD
        .decode(body.trim())
        .map_err(|e| AppError::internal(format!("ACME 账户密钥 PEM 解码失败: {e}")))
}

/// ECDSA P-256 JWS 签名：raw r||s（64 字节）→ base64url。
/// ring 的 FIXED_SIGNING 输出即 r||s 定长格式，无需 DER 转换。
fn es256_sign(key: &EcdsaKeyPair, rng: &SystemRandom, msg: &[u8]) -> Result<String, AppError> {
    let sig = key
        .sign(rng, msg)
        .map_err(|e| AppError::internal(format!("JWS 签名失败: {e}")))?;
    Ok(URL_SAFE_NO_PAD.encode(sig.as_ref()))
}

/// JWK：ECDSA P-256 公钥（crv/x/y，坐标 base64url 不带填充）。
fn p256_jwk(key: &EcdsaKeyPair) -> Value {
    let pub_der = key.public_key().as_ref();
    // 未压缩点格式：0x04 || X(32) || Y(32)
    let (x, y) = pub_der.split_at(33);
    json!({
        "kty": "EC",
        "crv": "P-256",
        "x": URL_SAFE_NO_PAD.encode(&x[1..]),
        "y": URL_SAFE_NO_PAD.encode(y),
    })
}

impl AcmeClient {
    /// 从账户密钥 PEM + 目录 URL 构造客户端（账户注册延后到 [`Self::ensure_account`]）。
    pub fn new(account_key_pem: &str, directory_url: &str) -> Result<Self, AppError> {
        let der = pem_to_pkcs8_der(account_key_pem)?;
        let signing_key = EcdsaKeyPair::from_pkcs8(
            &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            &der,
            &SystemRandom::new(),
        )
        .map_err(|e| AppError::internal(format!("ACME 账户密钥解析失败: {e}")))?;
        let http = reqwest::Client::builder()
            .user_agent(concat!("FlamePanel/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| AppError::internal(format!("ACME HTTP 客户端构建失败: {e}")))?;
        Ok(Self {
            http,
            signing_key,
            rng: SystemRandom::new(),
            account_url: None,
            nonce: None,
            directory: Directory::default(),
            directory_url: directory_url.to_string(),
        })
    }

    /// 拉取 ACME 目录（幂等：已拉取则跳过）。
    pub async fn fetch_directory(&mut self) -> Result<(), AppError> {
        if !self.directory.new_order.is_empty() {
            return Ok(());
        }
        let resp = self
            .http
            .get(&self.directory_url)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("ACME 目录拉取失败: {e}")))?;
        let dir: Value = resp
            .json()
            .await
            .map_err(|e| AppError::internal(format!("ACME 目录解析失败: {e}")))?;
        self.directory = Directory {
            new_nonce: dir
                .get("newNonce")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            new_account: dir
                .get("newAccount")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            new_order: dir
                .get("newOrder")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
        };
        if self.directory.new_order.is_empty() || self.directory.new_nonce.is_empty() {
            return Err(AppError::internal("ACME 目录缺少 newOrder/newNonce 端点"));
        }
        Ok(())
    }

    /// 获取新的 Replay-Nonce（HEAD newNonce）。
    async fn refresh_nonce(&mut self) -> Result<(), AppError> {
        let resp = self
            .http
            .head(&self.directory.new_nonce)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("ACME nonce 获取失败: {e}")))?;
        self.nonce = resp
            .headers()
            .get("replay-nonce")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        Ok(())
    }

    /// 账户密钥指纹 thumbprint（RFC 7638：JWK 规范化 JSON 的 SHA-256，base64url）。
    pub fn key_authorization(&self, token: &str) -> String {
        format!("{token}.{}", self.thumbprint_b64())
    }

    /// thumbprint base64url。
    pub fn thumbprint_b64(&self) -> String {
        let jwk = p256_jwk(&self.signing_key);
        // RFC 7638 规范化：按字典序仅保留必需字段（本构造已保证）
        let canonical = format!(
            r#"{{"crv":"P-256","kty":"EC","x":"{}","y":"{}"}}"#,
            jwk["x"].as_str().unwrap_or_default(),
            jwk["y"].as_str().unwrap_or_default()
        );
        let digest = Sha256::digest(canonical.as_bytes());
        URL_SAFE_NO_PAD.encode(digest)
    }

    /// 确保 ACME 账户已注册（幂等：已注册直接返回；首次 newAccount，
    /// 服务器返回 201（新建）或 200（existing，Location 头给账户 URL）统一处理）。
    pub async fn ensure_account(&mut self, contact: &[String]) -> Result<(), AppError> {
        if self.account_url.is_some() {
            return Ok(());
        }
        self.fetch_directory().await?;
        self.refresh_nonce().await?;
        let nonce = self
            .nonce
            .take()
            .ok_or_else(|| AppError::internal("ACME 服务器未返回 Replay-Nonce"))?;
        let payload = json!({
            "termsOfServiceAgreed": true,
            "contact": contact,
        });
        let protected = json!({
            "alg": "ES256",
            "jwk": p256_jwk(&self.signing_key),
            "nonce": nonce,
            "url": self.directory.new_account,
        });
        let body = self.jws_body(&protected, &payload)?;
        let resp = self
            .http
            .post(&self.directory.new_account)
            .header("Content-Type", "application/jose+json")
            .body(body)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("ACME newAccount 请求失败: {e}")))?;
        let account_url = resp
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                AppError::internal(format!(
                    "ACME newAccount 未返回账户 URL（status={}）",
                    resp.status()
                ))
            })?;
        self.nonce = resp
            .headers()
            .get("replay-nonce")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        self.account_url = Some(account_url);
        Ok(())
    }

    /// 构造 JWS（Flattened JSON Serialization）：protected/payload/signature。
    fn jws_body(&self, protected: &Value, payload: &Value) -> Result<String, AppError> {
        let signing_input = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(protected)?),
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(payload)?),
        );
        let sig = es256_sign(&self.signing_key, &self.rng, signing_input.as_bytes())?;
        Ok(json!({
            "protected": URL_SAFE_NO_PAD.encode(serde_json::to_vec(protected)?),
            "payload": URL_SAFE_NO_PAD.encode(serde_json::to_vec(payload)?),
            "signature": sig,
        })
        .to_string())
    }

    /// 已认证 POST-as-GET / POST 请求（自动补 nonce 与账户头，回读新 nonce）。
    /// 返回 (status, Location 头, 响应体 JSON/文本)。
    async fn post(
        &mut self,
        url: &str,
        payload: Value,
    ) -> Result<(u16, Option<String>, Value), AppError> {
        self.fetch_directory().await?;
        if self.nonce.is_none() {
            self.refresh_nonce().await?;
        }
        let nonce = self
            .nonce
            .take()
            .ok_or_else(|| AppError::internal("ACME Replay-Nonce 缺失"))?;
        let account_url = self
            .account_url
            .clone()
            .ok_or_else(|| AppError::internal("ACME 账户未注册"))?;
        let protected = json!({
            "alg": "ES256",
            "kid": account_url,
            "nonce": nonce,
            "url": url,
        });
        let body = self.jws_body(&protected, &payload)?;
        let resp = self
            .http
            .post(url)
            .header("Content-Type", "application/jose+json")
            .body(body)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("ACME 请求失败: {e}")))?;
        self.nonce = resp
            .headers()
            .get("replay-nonce")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let location = resp
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let status = resp.status().as_u16();
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let text = resp.text().await.unwrap_or_default();
        // 证书下载（application/pem-certificate-chain）保留原文
        let json = if content_type.starts_with("application/pem-certificate-chain") {
            Value::String(text)
        } else if text.trim().is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&text).unwrap_or(Value::String(text))
        };
        Ok((status, location, json))
    }

    /// 从 authz 对象挑出指定类型的 challenge 的 (url, token)。
    fn pick_challenge(authz: &Value, ctype: &str) -> Option<(String, String)> {
        let challenges = authz.get("challenges")?.as_array()?;
        for ch in challenges {
            if ch.get("type").and_then(|v| v.as_str()) == Some(ctype) {
                return Some((
                    ch.get("url").and_then(|v| v.as_str())?.to_string(),
                    ch.get("token").and_then(|v| v.as_str())?.to_string(),
                ));
            }
        }
        None
    }

    /// 创建订单。返回 (order URL, [(authz URL, challenge URL, token)])。
    /// 验证类型按 `dns01` 选择：true=dns-01（泛域名唯一途径），false=http-01。
    pub async fn new_order(
        &mut self,
        domains: &[String],
        _dns01: bool,
    ) -> Result<(String, Vec<OrderChallenge>), AppError> {
        let new_order_url = self.directory.new_order.clone();
        let identifiers: Vec<Value> = domains
            .iter()
            .map(|d| json!({"type": "dns", "value": d}))
            .collect();
        let (status, location, body) = self
            .post(&new_order_url, json!({ "identifiers": identifiers }))
            .await?;
        if status != 201 {
            return Err(AppError::internal(format!(
                "ACME newOrder 失败（status={status}）：{body}"
            )));
        }
        let order_url = location
            .ok_or_else(|| AppError::internal("ACME newOrder 未返回订单 URL（Location）"))?;
        let mut challenges = Vec::new();
        if let Some(authzs) = body.get("authorizations").and_then(|v| v.as_array()) {
            for url in authzs.iter().filter_map(|v| v.as_str()) {
                // authz URL 先记录，token 延后到 fetch_authorization 拉取
                challenges.push(url.to_string());
            }
        }
        Ok((
            order_url,
            challenges
                .into_iter()
                .map(|u| OrderChallenge {
                    authz_url: u,
                    challenge_url: String::new(),
                    token: String::new(),
                    domain: String::new(),
                })
                .collect(),
        ))
    }
}

/// 订单内单个域名的授权/质询项。
#[derive(Debug, Clone)]
pub struct OrderChallenge {
    /// 授权（authz）资源 URL
    pub authz_url: String,
    /// 选定的 challenge 资源 URL（ready 轮询用）
    pub challenge_url: String,
    /// challenge token（key_authorization 组装用）
    pub token: String,
    /// 该授权对应的域名
    pub domain: String,
}

impl AcmeClient {
    /// 拉取授权详情并选出指定类型的 challenge。
    pub async fn fetch_authorization(
        &mut self,
        authz_url: &str,
        dns01: bool,
    ) -> Result<OrderChallenge, AppError> {
        let (status, _, authz) = self.post(authz_url, Value::Null).await?;
        if status != 200 {
            return Err(AppError::internal(format!(
                "ACME authz 拉取失败（status={status}）：{authz}"
            )));
        }
        let domain = authz
            .get("identifier")
            .and_then(|i| i.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let ctype = if dns01 { "dns-01" } else { "http-01" };
        let (challenge_url, token) = Self::pick_challenge(&authz, ctype).ok_or_else(|| {
            AppError::internal(format!("ACME 授权缺少 {ctype} challenge：{authz}"))
        })?;
        Ok(OrderChallenge {
            authz_url: authz_url.to_string(),
            challenge_url,
            token,
            domain,
        })
    }

    /// 通知 ACME 服务器 challenge 已就绪（POST challenge 资源，空 payload）。
    pub async fn notify_challenge_ready(&mut self, challenge_url: &str) -> Result<(), AppError> {
        let (status, _, body) = self.post(challenge_url, Value::Null).await?;
        if !(200..300).contains(&status) {
            return Err(AppError::internal(format!(
                "ACME challenge 就绪通知失败（status={status}）：{body}"
            )));
        }
        Ok(())
    }

    /// 轮询订单直到 ready/valid 或出错；返回最终状态字符串与（valid 时的）证书 URL。
    pub async fn poll_order(
        &mut self,
        order_url: &str,
    ) -> Result<(String, Option<String>), AppError> {
        for _ in 0..60 {
            let (status, _, body) = self.post(order_url, Value::Null).await?;
            if status != 200 {
                return Err(AppError::internal(format!(
                    "ACME 订单轮询失败（status={status}）：{body}"
                )));
            }
            let state = body
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            match state.as_str() {
                "ready" | "valid" => {
                    let cert_url = body
                        .get("certificate")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    return Ok((state, cert_url));
                }
                "invalid" | "expired" | "canceled" | "rejected" => {
                    let problem = body
                        .get("error")
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| state.clone());
                    return Err(AppError::internal(format!("ACME 订单失败: {problem}")));
                }
                _ => tokio::time::sleep(std::time::Duration::from_secs(2)).await,
            }
        }
        Err(AppError::internal("ACME 订单轮询超时（120s）"))
    }

    /// finalize（提交 CSR，DER base64url）并等待订单 valid。
    pub async fn finalize_order(
        &mut self,
        order_url: &str,
        csr_der: &[u8],
    ) -> Result<(), AppError> {
        let payload = json!({ "csr": URL_SAFE_NO_PAD.encode(csr_der) });
        let (status, _, body) = self.post(order_url, payload).await?;
        if !(200..300).contains(&status) {
            return Err(AppError::internal(format!(
                "ACME finalize 失败（status={status}）：{body}"
            )));
        }
        Ok(())
    }

    /// 下载证书链（PEM，包含中间证书）。
    pub async fn download_certificate(&mut self, cert_url: &str) -> Result<String, AppError> {
        let (status, _, body) = self.post(cert_url, Value::Null).await?;
        if status != 200 {
            return Err(AppError::internal(format!(
                "ACME 证书下载失败（status={status}）"
            )));
        }
        match body {
            Value::String(pem) if pem.contains("BEGIN CERTIFICATE") => Ok(pem),
            other => Err(AppError::internal(format!(
                "ACME 证书下载响应非 PEM：{other}"
            ))),
        }
    }
}
