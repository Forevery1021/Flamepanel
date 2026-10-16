//! 验证质询组装（M-B 主线三 §6.1 · Stage 2）。
//!
//! - HTTP-01：令牌放置 `<webroot>/.well-known/acme-challenge/<token>`（文件落盘，
//!   由既有 Web 引擎静态服务；内容 = key_authorization）
//! - DNS-01：TXT 记录值 = base64url(SHA-256(key_authorization))（RFC 8555 §8.4）

use crate::core::error::AppError;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};

/// HTTP-01 验证文件相对目录（`.well-known/acme-challenge`）。
pub const HTTP01_CHALLENGE_DIR: &str = ".well-known/acme-challenge";

/// DNS-01 TXT 记录名前缀。
pub const DNS01_PREFIX: &str = "_acme-challenge.";

/// HTTP-01：把 key_authorization 写入 webroot 验证文件，返回文件路径。
pub fn write_http01_file(
    webroot: &std::path::Path,
    token: &str,
    key_authorization: &str,
) -> Result<std::path::PathBuf, AppError> {
    let dir = webroot.join(HTTP01_CHALLENGE_DIR);
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::internal(format!("创建 HTTP-01 验证目录失败: {e}")))?;
    let file = dir.join(token);
    // token 白名单由调用方校验（ACME 服务器 token 本身是 base64url）；
    // 此处防路径穿越二次把关
    if token.contains('/') || token.contains('\\') || token.contains("..") {
        return Err(AppError::BadRequest("非法 challenge token".into()));
    }
    std::fs::write(&file, key_authorization)
        .map_err(|e| AppError::internal(format!("写入 HTTP-01 验证文件失败: {e}")))?;
    Ok(file)
}

/// HTTP-01：清理验证文件（失败不阻断——验证已完成/放弃，残留不影响安全）。
pub fn cleanup_http01_file(webroot: &std::path::Path, token: &str) {
    let _ = std::fs::remove_file(webroot.join(HTTP01_CHALLENGE_DIR).join(token));
}

/// DNS-01：TXT 记录值。
pub fn dns01_txt_value(key_authorization: &str) -> String {
    let digest = Sha256::digest(key_authorization.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http01_roundtrip_and_traversal_guard() {
        let dir = std::env::temp_dir().join(format!("fp-http01-{}", uuid::Uuid::new_v4()));
        let file = write_http01_file(&dir, "tok123", "tok123.thumb").unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "tok123.thumb");
        assert!(file.ends_with(".well-known/acme-challenge/tok123"));
        cleanup_http01_file(&dir, "tok123");
        assert!(!file.exists());
        // 路径穿越 token 拒绝
        assert!(write_http01_file(&dir, "../evil", "x").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dns01_txt_value_is_sha256_b64url() {
        let v = dns01_txt_value("token.thumb");
        assert_eq!(v.len(), 43); // SHA-256 → base64url 无填充恒 43 字符
                                 // 同输入同输出
        assert_eq!(v, dns01_txt_value("token.thumb"));
    }
}
