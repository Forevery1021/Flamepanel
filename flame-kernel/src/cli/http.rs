//! 在线调用：经本机回环访问面板 REST API。
//!
//! 身份：用面板同一把 JWT 密钥现签 **60 s** access token（`sub` = 选定管理员），
//! 不落盘、不复用；API 侧仍走完整的鉴权 / 权限 / 审计链路。

use std::time::Duration;

use serde_json::{json, Value};

use crate::utils::jwt::JwtUtils;

use super::context::Ctx;
use super::db::{pick_user, Db};
use super::output::CliError;

/// CLI 令牌有效期（秒）。
pub const TOKEN_TTL_SECS: u64 = 60;

pub struct ApiResponse {
    pub status: u16,
    pub body: Value,
    pub request_id: Option<String>,
}

pub struct Client {
    http: reqwest::Client,
    pub base: String,
}

impl Client {
    pub fn new(ctx: &Ctx) -> Result<Self, CliError> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(120))
            .user_agent(concat!("fpctl/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| CliError::general(format!("http client: {e}")))?;
        Ok(Self {
            http,
            base: ctx.base_url(),
        })
    }

    fn transport_error(&self, e: reqwest::Error) -> CliError {
        if e.is_connect() || e.is_timeout() {
            CliError::not_running(format!(
                "panel not reachable at {} ({e}); check `fpctl status`",
                self.base
            ))
        } else {
            CliError::general(format!("request to {} failed: {e}", self.base))
        }
    }

    /// `GET /api/health`（免认证）。连不上 → 退出码 3。
    pub async fn health(&self) -> Result<Value, CliError> {
        let r = self
            .http
            .get(format!("{}/api/health", self.base))
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| self.transport_error(e))?;
        let status = r.status();
        let body: Value = r.json().await.unwrap_or(Value::Null);
        if !status.is_success() && body.is_null() {
            return Err(CliError::general(format!("/api/health returned {status}")));
        }
        Ok(body)
    }

    pub async fn request(
        &self,
        method: &str,
        path: &str,
        token: Option<&str>,
        body: Option<Value>,
    ) -> Result<ApiResponse, CliError> {
        let m = reqwest::Method::from_bytes(method.to_ascii_uppercase().as_bytes())
            .map_err(|_| CliError::usage(format!("invalid HTTP method '{method}'")))?;
        let mut req = self.http.request(m, format!("{}{}", self.base, path));
        if let Some(t) = token {
            req = req.bearer_auth(t);
        }
        if let Some(b) = body {
            req = req.json(&b);
        }
        let r = req.send().await.map_err(|e| self.transport_error(e))?;
        let status = r.status().as_u16();
        let request_id = r
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .map(String::from);
        let bytes = r
            .bytes()
            .await
            .map_err(|e| CliError::general(format!("reading response: {e}")))?;
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()))
        };
        Ok(ApiResponse {
            status,
            body,
            request_id,
        })
    }
}

/// 为选定用户（缺省首个管理员）现签短期令牌。
pub async fn sign_token(ctx: &Ctx, user: Option<&str>) -> Result<(String, String), CliError> {
    let secret = ctx.jwt_secret().ok_or_else(|| {
        CliError::permission(
            "cannot read the JWT secret (data/jwt_secret.key or OP_JWT_SECRET); run as root or the service user",
        )
    })?;
    let db = Db::open(ctx).await?;
    let picked = pick_user(&db.users(), user).await;
    db.close().await;
    let u = picked?;
    let token = JwtUtils::new_pair(&secret).sign_access_with_ttl(u.id, TOKEN_TTL_SECS)?;
    Ok((token, u.username))
}

/// HTTP 状态 → 退出码；成功返回响应体。错误体（`{code,error,message}`）原样放进 detail。
pub fn into_result(resp: ApiResponse) -> Result<Value, CliError> {
    if (200..300).contains(&resp.status) {
        return Ok(resp.body);
    }
    let api_code = resp.body.get("error").and_then(Value::as_str).unwrap_or("");
    let api_msg = resp
        .body
        .get("message")
        .and_then(Value::as_str)
        .map(String::from)
        .unwrap_or_else(|| resp.body.to_string());
    let mut msg = format!("HTTP {} {api_code}: {api_msg}", resp.status);
    if api_code == "PASSWORD_CHANGE_REQUIRED" {
        msg.push_str(" — log in to the panel and change the password first");
    }
    if resp.status >= 500 {
        if let Some(rid) = &resp.request_id {
            msg.push_str(&format!(" — see `fpctl logs --grep {rid}`"));
        }
    }
    let err = match resp.status {
        401 | 403 => CliError::permission(msg),
        404 => CliError::not_found(msg),
        400 | 422 => CliError::usage(msg),
        _ => CliError::general(msg),
    };
    Err(err.with_detail(json!({
        "status": resp.status,
        "request_id": resp.request_id,
        "body": resp.body,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::output::ExitCode;

    fn resp(status: u16, body: Value) -> ApiResponse {
        ApiResponse {
            status,
            body,
            request_id: Some("rid-1".into()),
        }
    }

    #[test]
    fn status_mapping() {
        assert_eq!(into_result(resp(200, json!({"a":1}))).unwrap()["a"], 1);
        let cases = [
            (401, ExitCode::Permission),
            (403, ExitCode::Permission),
            (404, ExitCode::NotFound),
            (400, ExitCode::Usage),
            (409, ExitCode::General),
            (500, ExitCode::General),
        ];
        for (s, code) in cases {
            let e =
                into_result(resp(s, json!({"code": s, "error": "X", "message": "m"}))).unwrap_err();
            assert_eq!(e.exit, code, "{s}");
            assert_eq!(e.detail.as_ref().unwrap()["status"], s);
        }
    }

    #[test]
    fn server_errors_point_to_logs_and_password_hint() {
        let e = into_result(resp(
            500,
            json!({"error":"INTERNAL","message":"internal server error"}),
        ))
        .unwrap_err();
        assert!(e.message.contains("fpctl logs --grep rid-1"));
        let e = into_result(resp(
            403,
            json!({"error":"PASSWORD_CHANGE_REQUIRED","message":"m"}),
        ))
        .unwrap_err();
        assert!(e.message.contains("change the password"));
    }
}
