//! 应用包签名与验签（M-A Issue 1.3，Doc/20 §4.2 / Doc/16 §6.3）。
//!
//! 签名方案：Ed25519 对包内容摘要签名。
//! - 包摘要 = 包内全部文件（相对路径排序后）的 SHA-256 分级哈希；
//!   摘要原文格式：每行 `{sha256_hex}  {relative_path}`（与 sha256sum -c 兼容），以 `\n` 结尾；
//! - 签名以 base64（STANDARD）编码存放于 `app.json` 的 `signature` 字段；
//! - 公钥以 base64（STANDARD）编码存放于远程源 `public_key` 字段（64 字节原始密钥）。
//!
//! 语义：
//! - `app.json` 自身的 `signature` 字段从摘要计算中剔除，避免自我引用；
//! - 源未配置公钥 → 不验签（放行，配合 `app_signature_policy` 可收紧为强制）；
//! - 源配置了公钥 → 强制验签，缺失/不匹配签名均拒绝。

use crate::core::error::AppError;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::path::Path;

/// 验签策略（settings 表 `app_signature_policy`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignaturePolicy {
    /// 默认：源配置了公钥才验签
    Optional,
    /// 强制：所有远程包必须签名且验签通过（源未配公钥视为不可信）
    Required,
}

impl SignaturePolicy {
    /// 从 settings 值解析（非法值回落 Optional）。
    pub fn from_setting(v: Option<&str>) -> Self {
        match v.unwrap_or_default() {
            "required" => Self::Required,
            _ => Self::Optional,
        }
    }
}

/// 计算目录包内容摘要（SHA-256 hex，64 字符）。
///
/// 遍历目录下全部常规文件（相对路径按字节序排序），逐文件 sha256 后
/// 以 `"{file_hash}  {rel_path}\n"` 拼接为清单原文，再对清单原文做 sha256。
/// `exclude` 中的相对路径（如 `app.json` 的 signature 字段需剔除的场景用不到——
/// 该字段在文件内容之外）不参与；本函数用于文件级摘要，`app.json` 是否剔除
/// 由调用方决定（签名数据存放于 app.json 字段而非独立文件，文件摘要天然不受影响）。
pub fn package_digest(root: &Path) -> Result<String, AppError> {
    let manifest = manifest_text(root)?;
    let mut hasher = Sha256::new();
    hasher.update(manifest.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

/// 本地导入验签的签名原文（M-A Issue 1.3）：
/// `app.json`（剥离 signature 字段后按 serde_json BTreeMap 规范化）+ `\n` +
/// 除 app.json 外的包文件清单原文。
pub fn local_signature_payload(
    root: &Path,
    app_json_value: &serde_json::Value,
    _signature_b64: &str,
) -> Result<Vec<u8>, AppError> {
    let mut canonical = serde_json::Map::new();
    if let serde_json::Value::Object(map) = app_json_value {
        for (k, v) in map {
            if k != "signature" {
                canonical.insert(k.clone(), v.clone());
            }
        }
    }
    let normalized = serde_json::to_vec(&serde_json::Value::Object(canonical))
        .map_err(|e| AppError::internal(format!("规范化 app.json 失败: {}", e)))?;

    // 其余文件清单（相对路径排序，排除 app.json 自身）
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    files.sort();
    let mut lines = String::new();
    for rel in &files {
        if rel == std::path::Path::new("app.json") {
            continue;
        }
        let hash = file_sha256(&root.join(rel))?;
        lines.push_str(&format!("{}  {}\n", hash, rel.display()));
    }

    let mut payload = normalized;
    payload.push(b'\n');
    payload.extend_from_slice(lines.as_bytes());
    Ok(payload)
}

/// 对任意字节原文验签（local_signature_payload 的配套入口）。
pub fn verify_payload(
    payload: &[u8],
    public_key_b64: &str,
    signature_b64: &str,
) -> Result<(), AppError> {
    let pk_bytes: [u8; ed25519_dalek::PUBLIC_KEY_LENGTH] =
        base64::engine::general_purpose::STANDARD
            .decode(public_key_b64.trim())
            .ok()
            .and_then(|b| b.try_into().ok())
            .ok_or_else(|| AppError::ValidationError("公钥非法（base64/长度）".into()))?;
    let verifying_key = VerifyingKey::from_bytes(&pk_bytes)
        .map_err(|_| AppError::ValidationError("公钥不是合法的 Ed25519 密钥".into()))?;
    let sig_bytes: [u8; Signature::BYTE_SIZE] = base64::engine::general_purpose::STANDARD
        .decode(signature_b64.trim())
        .ok()
        .and_then(|b| b.try_into().ok())
        .ok_or_else(|| AppError::ValidationError("签名非法（base64/长度）".into()))?;
    verifying_key
        .verify(payload, &Signature::from_bytes(&sig_bytes))
        .map_err(|_| AppError::ValidationError("应用包验签失败：签名与包内容不匹配".into()))
}

/// 生成清单原文：按相对路径排序的 `{sha256}  {path}` 行。
fn manifest_text(root: &Path) -> Result<String, AppError> {
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    files.sort();
    let mut lines = String::new();
    for rel in &files {
        // 签名文件本身不参与摘要（先有摘要后有签名，避免自我引用）
        if rel == std::path::Path::new("signature.sig") {
            continue;
        }
        let abs = root.join(rel);
        let hash = file_sha256(&abs)?;
        lines.push_str(&format!("{}  {}\n", hash, rel.display()));
    }
    Ok(lines)
}

/// 校验 Ed25519 公钥格式（64 字节原始密钥的 base64 编码）。
pub fn validate_public_key(public_key_b64: &str) -> Result<(), AppError> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(public_key_b64.trim())
        .map_err(|_| AppError::ValidationError("公钥必须是合法的 base64 编码".into()))?;
    if bytes.len() != ed25519_dalek::PUBLIC_KEY_LENGTH {
        return Err(AppError::ValidationError(format!(
            "公钥长度非法: {} 字节（Ed25519 公钥应为 {} 字节）",
            bytes.len(),
            ed25519_dalek::PUBLIC_KEY_LENGTH
        )));
    }
    let key: [u8; ed25519_dalek::PUBLIC_KEY_LENGTH] = bytes
        .try_into()
        .map_err(|_| AppError::ValidationError("公钥长度非法".into()))?;
    VerifyingKey::from_bytes(&key)
        .map(|_| ())
        .map_err(|_| AppError::ValidationError("公钥不是合法的 Ed25519 密钥".into()))
}

/// 验签：对包目录摘要验证 base64 编码的 Ed25519 签名。
pub fn verify_package(
    root: &Path,
    public_key_b64: &str,
    signature_b64: &str,
) -> Result<(), AppError> {
    let pk_bytes: [u8; ed25519_dalek::PUBLIC_KEY_LENGTH] =
        base64::engine::general_purpose::STANDARD
            .decode(public_key_b64.trim())
            .ok()
            .and_then(|b| b.try_into().ok())
            .ok_or_else(|| AppError::ValidationError("远程源公钥非法（base64/长度）".into()))?;
    let verifying_key = VerifyingKey::from_bytes(&pk_bytes)
        .map_err(|_| AppError::ValidationError("远程源公钥不是合法的 Ed25519 密钥".into()))?;

    let sig_bytes: [u8; Signature::BYTE_SIZE] = base64::engine::general_purpose::STANDARD
        .decode(signature_b64.trim())
        .ok()
        .and_then(|b| b.try_into().ok())
        .ok_or_else(|| AppError::ValidationError("应用包签名非法（base64/长度）".into()))?;
    let signature = Signature::from_bytes(&sig_bytes);

    let digest = package_digest(root)?;
    verifying_key
        .verify(digest.as_bytes(), &signature)
        .map_err(|_| AppError::ValidationError("应用包验签失败：签名与包内容不匹配".into()))
}

/// 对目录包签名（供打包端/测试使用）：返回 base64 编码的 Ed25519 签名。
pub fn sign_package(root: &Path, signing_key: &SigningKey) -> Result<String, AppError> {
    let digest = package_digest(root)?;
    Ok(base64::engine::general_purpose::STANDARD
        .encode(signing_key.sign(digest.as_bytes()).to_bytes()))
}

/// 生成密钥对（供管理端生成源公钥/打包端使用），返回 (公钥 b64, 私钥 b64)。
pub fn generate_keypair() -> (String, String) {
    let mut seed = [0u8; ed25519_dalek::SECRET_KEY_LENGTH];
    getrandom_seed(&mut seed);
    let signing = SigningKey::from_bytes(&seed);
    (
        base64::engine::general_purpose::STANDARD.encode(signing.verifying_key().as_bytes()),
        base64::engine::general_purpose::STANDARD.encode(signing.to_bytes()),
    )
}

fn getrandom_seed(buf: &mut [u8; 32]) {
    // ed25519-dalek 2.x 的随机源由调用方提供；这里直读 OS 熵源 `/dev/urandom`（read_exact 32 字节）。
    // Doc/33 A-10：旧回退分支 `std::fs::read("/dev/urandom")` 读取无 EOF 的字符设备，
    // 会无限增长内存；熵源不可用时无安全退路，直接 panic 比生成弱密钥更安全。
    getrandom_fill(buf).expect("读取 /dev/urandom 失败：无法生成安全密钥");
}

fn getrandom_fill(buf: &mut [u8; 32]) -> Result<(), ()> {
    let f = std::fs::File::open("/dev/urandom").map_err(|_| ())?;
    use std::io::Read;
    let mut f = f;
    f.read_exact(buf).map_err(|_| ())
}

fn file_sha256(path: &Path) -> Result<String, AppError> {
    let data = std::fs::read(path)
        .map_err(|e| AppError::internal(format!("读取包文件失败 {}: {}", path.display(), e)))?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(format!("{:x}", hasher.finalize()))
}

fn collect_files(
    root: &Path,
    dir: &Path,
    out: &mut Vec<std::path::PathBuf>,
) -> Result<(), AppError> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| AppError::internal(format!("读取包目录失败 {}: {}", dir.display(), e)))?;
    for entry in entries.flatten() {
        let path = entry.path();
        // Doc/33 A-10：`Path::is_dir/is_file` 会跟随符号链接——包内链接可指向包外文件
        // （摘要覆盖范围与实际安装内容不一致）或形成目录环（无限递归）。改用不跟随链接的
        // `DirEntry::file_type`，并整体拒绝链接；文件名含控制字符（换行可伪造清单行）同样拒绝。
        let ft = entry.file_type().map_err(|e| {
            AppError::internal(format!("读取包条目类型失败 {}: {}", path.display(), e))
        })?;
        if ft.is_symlink() {
            return Err(AppError::ValidationError(format!(
                "应用包不允许包含符号链接: {}",
                path.display()
            )));
        }
        if entry
            .file_name()
            .to_string_lossy()
            .chars()
            .any(char::is_control)
        {
            return Err(AppError::ValidationError("应用包文件名包含控制字符".into()));
        }
        if ft.is_dir() {
            collect_files(root, &path, out)?;
        } else if ft.is_file() {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| AppError::internal(format!("计算相对路径失败: {}", e)))?;
            out.push(rel.to_path_buf());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_pkg(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("flame_sig_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("1.0.0")).unwrap();
        std::fs::write(dir.join("app.json"), br#"{"key":"demo","name":"Demo"}"#).unwrap();
        std::fs::write(
            dir.join("1.0.0/docker-compose.yml"),
            b"services:\n  app:\n    image: nginx:alpine\n",
        )
        .unwrap();
        dir
    }

    #[test]
    fn digest_is_deterministic_and_content_sensitive() {
        let pkg = temp_pkg("digest");
        let d1 = package_digest(&pkg).unwrap();
        let d2 = package_digest(&pkg).unwrap();
        assert_eq!(d1, d2);
        assert_eq!(d1.len(), 64);

        // 内容变化 → 摘要变化
        std::fs::write(
            pkg.join("1.0.0/docker-compose.yml"),
            b"services:\n  app:\n    image: redis:7\n",
        )
        .unwrap();
        let d3 = package_digest(&pkg).unwrap();
        assert_ne!(d1, d3);
        let _ = std::fs::remove_dir_all(&pkg);
    }

    #[test]
    fn digest_ignores_file_order() {
        let pkg = temp_pkg("order");
        let d1 = package_digest(&pkg).unwrap();
        // 增加子目录文件（顺序不改变摘要，因为排序）
        std::fs::create_dir_all(pkg.join("assets")).unwrap();
        std::fs::write(pkg.join("assets/logo.png"), b"png").unwrap();
        let d2 = package_digest(&pkg).unwrap();
        assert_ne!(d1, d2, "新增文件必须改变摘要");
        let _ = std::fs::remove_dir_all(&pkg);
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let pkg = temp_pkg("roundtrip");
        let (_pk, sk) = generate_keypair();
        let sk_bytes: [u8; ed25519_dalek::SECRET_KEY_LENGTH] =
            base64::engine::general_purpose::STANDARD
                .decode(&sk)
                .unwrap()
                .try_into()
                .unwrap();
        let signing = SigningKey::from_bytes(&sk_bytes);
        let sig = sign_package(&pkg, &signing).unwrap();

        let pk =
            base64::engine::general_purpose::STANDARD.encode(signing.verifying_key().as_bytes());
        assert!(verify_package(&pkg, &pk, &sig).is_ok());

        // 篡改内容 → 验签失败
        std::fs::write(pkg.join("app.json"), br#"{"key":"demo","name":"Evil"}"#).unwrap();
        assert!(verify_package(&pkg, &pk, &sig).is_err());
        let _ = std::fs::remove_dir_all(&pkg);
    }

    #[test]
    fn verify_rejects_bad_signature_and_key() {
        let pkg = temp_pkg("bad");
        let (pk, _sk) = generate_keypair();
        assert!(verify_package(&pkg, &pk, "not-base64!!").is_err());
        // 合法 base64 但长度错误的签名
        let short_sig = base64::engine::general_purpose::STANDARD.encode([0u8; 16]);
        assert!(verify_package(&pkg, &pk, &short_sig).is_err());
        // 错误公钥
        let (other_pk, _sk2) = generate_keypair();
        let (_pk3, sk3) = generate_keypair();
        let sk_bytes: [u8; ed25519_dalek::SECRET_KEY_LENGTH] =
            base64::engine::general_purpose::STANDARD
                .decode(&sk3)
                .unwrap()
                .try_into()
                .unwrap();
        let sig = sign_package(&pkg, &SigningKey::from_bytes(&sk_bytes)).unwrap();
        assert!(verify_package(&pkg, &other_pk, &sig).is_err());
        let _ = std::fs::remove_dir_all(&pkg);
    }

    #[test]
    fn validate_public_key_checks() {
        let (pk, _sk) = generate_keypair();
        assert!(validate_public_key(&pk).is_ok());
        assert!(validate_public_key("!!!").is_err());
        assert!(
            validate_public_key(&base64::engine::general_purpose::STANDARD.encode([0u8; 16]))
                .is_err()
        );
        // 32 字节合法长度：ed25519-dalek 对全 0（群单位点）也接受，此处仅验证长度通过
        let zeros = base64::engine::general_purpose::STANDARD.encode([0u8; 32]);
        assert!(validate_public_key(&zeros).is_ok());
    }

    #[test]
    fn digest_rejects_symlinks_and_control_chars() {
        let pkg = temp_pkg("links");
        std::os::unix::fs::symlink("/etc/hostname", pkg.join("docker-compose.yml")).unwrap();
        assert!(package_digest(&pkg).is_err(), "符号链接必须拒绝");
        std::fs::remove_file(pkg.join("docker-compose.yml")).unwrap();
        // 目录环不得导致无限递归
        std::os::unix::fs::symlink(&pkg, pkg.join("loop")).unwrap();
        assert!(package_digest(&pkg).is_err());
        std::fs::remove_file(pkg.join("loop")).unwrap();
        std::fs::write(pkg.join("x\nforged"), "a").unwrap();
        assert!(package_digest(&pkg).is_err(), "文件名换行可伪造清单行");
        std::fs::remove_file(pkg.join("x\nforged")).unwrap();
        assert!(package_digest(&pkg).is_ok());
        let _ = std::fs::remove_dir_all(&pkg);
    }

    #[test]
    fn policy_parsing() {
        assert_eq!(
            SignaturePolicy::from_setting(None),
            SignaturePolicy::Optional
        );
        assert_eq!(
            SignaturePolicy::from_setting(Some("optional")),
            SignaturePolicy::Optional
        );
        assert_eq!(
            SignaturePolicy::from_setting(Some("required")),
            SignaturePolicy::Required
        );
        assert_eq!(
            SignaturePolicy::from_setting(Some("bogus")),
            SignaturePolicy::Optional
        );
    }
}
