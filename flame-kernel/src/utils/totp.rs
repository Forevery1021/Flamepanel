//! TOTP（RFC 6238）/ HOTP（RFC 4226）与恢复码工具——两步验证（Doc/33 排除项「TOTP 2FA」）。
//!
//! 仅依赖既有直接依赖（`hmac` + `sha1` + `sha2`），base32 手写（RFC 4648，无填充）。
//! 参数取认证器 App 的通用缺省：SHA-1、6 位、30 秒步长；校验容忍前后各 1 个步长的时钟偏差。

use hmac::{Hmac, Mac};
use sha1::Sha1;
use sha2::{Digest, Sha256};

/// 步长（秒）
pub const STEP_SECS: u64 = 30;
/// 验证码位数
pub const DIGITS: u32 = 6;
/// 允许的时钟偏差（步数，前后各）
pub const SKEW_STEPS: u64 = 1;
/// 密钥字节数（160 bit，RFC 4226 推荐长度）
pub const SECRET_BYTES: usize = 20;
/// 恢复码数量
pub const RECOVERY_CODE_COUNT: usize = 10;

const B32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// base32 编码（大写、无填充）。
pub fn base32_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(5) * 8);
    let mut buf: u64 = 0;
    let mut bits = 0u32;
    for &b in data {
        buf = (buf << 8) | b as u64;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(B32[((buf >> bits) & 0x1f) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(B32[((buf << (5 - bits)) & 0x1f) as usize] as char);
    }
    out
}

/// base32 解码：忽略空格 / 连字符 / 填充，大小写不敏感；含非法字符返回 `None`。
pub fn base32_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 5 / 8);
    let mut buf: u64 = 0;
    let mut bits = 0u32;
    for c in s.chars() {
        if c == ' ' || c == '-' || c == '=' {
            continue;
        }
        let u = c.to_ascii_uppercase() as u8;
        let v = B32.iter().position(|&x| x == u)? as u64;
        buf = (buf << 5) | v;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xff) as u8);
        }
    }
    Some(out)
}

/// CSPRNG 字节（熵源复用 uuid v4 → getrandom，与 `secret_box` 一致，不引入新依赖）。
fn random_bytes(n: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(n + 16);
    while out.len() < n {
        out.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    }
    out.truncate(n);
    out
}

/// 生成新的 base32 密钥。
pub fn generate_secret() -> String {
    base32_encode(&random_bytes(SECRET_BYTES))
}

/// HOTP：HMAC-SHA1 + 动态截断（RFC 4226 §5.3）。
pub fn hotp(secret: &[u8], counter: u64) -> u32 {
    // HMAC 接受任意长度密钥，new_from_slice 不会失败
    let mut mac = match Hmac::<Sha1>::new_from_slice(secret) {
        Ok(m) => m,
        Err(_) => return u32::MAX,
    };
    mac.update(&counter.to_be_bytes());
    let h = mac.finalize().into_bytes();
    let off = (h[h.len() - 1] & 0x0f) as usize;
    let bin = ((h[off] as u32 & 0x7f) << 24)
        | ((h[off + 1] as u32) << 16)
        | ((h[off + 2] as u32) << 8)
        | (h[off + 3] as u32);
    bin % 10u32.pow(DIGITS)
}

/// 指定 Unix 秒所在的步号。
pub fn step_at(unix_secs: u64) -> u64 {
    unix_secs / STEP_SECS
}

/// 规范化用户输入的验证码：去空白，必须恰为 6 位数字。
fn parse_code(code: &str) -> Option<u32> {
    let c: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    if c.len() != DIGITS as usize || !c.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    c.parse().ok()
}

/// 校验验证码；成功返回命中的步号。
///
/// `last_step`：该用户最近一次成功使用的步号——命中步号必须**严格大于**它（防重放：
/// 同一验证码在有效窗口内不能被第二次使用）。
pub fn verify(secret_b32: &str, code: &str, unix_secs: u64, last_step: Option<u64>) -> Option<u64> {
    let secret = base32_decode(secret_b32)?;
    if secret.is_empty() {
        return None;
    }
    let want = parse_code(code)?;
    let now = step_at(unix_secs);
    let mut hit = None;
    // 遍历全部候选再返回，避免按命中位置提前退出带来的时序差异
    for step in now.saturating_sub(SKEW_STEPS)..=now + SKEW_STEPS {
        let ok = crate::utils::constant_time_eq(
            format!("{:06}", hotp(&secret, step)).as_bytes(),
            format!("{:06}", want).as_bytes(),
        );
        if ok && last_step.is_none_or(|l| step > l) && hit.is_none() {
            hit = Some(step);
        }
    }
    hit
}

/// otpauth URI（认证器 App 扫码）。
pub fn otpauth_uri(issuer: &str, account: &str, secret_b32: &str) -> String {
    let label = format!("{}:{}", pct(issuer), pct(account));
    format!(
        "otpauth://totp/{label}?secret={secret_b32}&issuer={}&algorithm=SHA1&digits={DIGITS}&period={STEP_SECS}",
        pct(issuer)
    )
}

/// 最小百分号编码（URI 组件，保留非保留字符）。
fn pct(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

/// 恢复码字符集（无歧义：去掉 0/O、1/I/L）。
const RC_CHARSET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

/// 生成一组恢复码（`XXXXX-XXXXX`，约 49 bit 熵/个）。
pub fn generate_recovery_codes() -> Vec<String> {
    (0..RECOVERY_CODE_COUNT)
        .map(|_| {
            let bytes = random_bytes(10);
            let chars: String = bytes
                .iter()
                // 31 个字符：256 % 31 = 8，偏差可忽略（恢复码为一次性、带锁定的低频凭证）
                .map(|b| RC_CHARSET[(*b as usize) % RC_CHARSET.len()] as char)
                .collect();
            format!("{}-{}", &chars[..5], &chars[5..])
        })
        .collect()
}

/// 规范化恢复码（去空白 / 连字符，转大写），用于哈希比对。
pub fn normalize_recovery_code(code: &str) -> String {
    code.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// 恢复码哈希（SHA-256 十六进制；库内只存哈希）。
pub fn hash_recovery_code(code: &str) -> String {
    let d = Sha256::digest(normalize_recovery_code(code).as_bytes());
    d.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 6238 附录 B 的 SHA-1 种子（ASCII "12345678901234567890"）。
    const RFC_SEED: &[u8] = b"12345678901234567890";

    #[test]
    fn base32_roundtrip_and_rfc4648_vectors() {
        assert_eq!(base32_encode(b""), "");
        assert_eq!(base32_encode(b"f"), "MY");
        assert_eq!(base32_encode(b"foobar"), "MZXW6YTBOI");
        assert_eq!(base32_decode("mzxw6ytboi").unwrap(), b"foobar");
        assert_eq!(base32_decode("MZXW 6YTB-OI==").unwrap(), b"foobar");
        assert!(base32_decode("MZ1W").is_none());
        let s = generate_secret();
        assert_eq!(base32_decode(&s).unwrap().len(), SECRET_BYTES);
    }

    #[test]
    fn hotp_matches_rfc4226_appendix_d() {
        let expect = [755224, 287082, 359152, 969429, 338314];
        for (i, e) in expect.iter().enumerate() {
            assert_eq!(hotp(RFC_SEED, i as u64), *e);
        }
    }

    #[test]
    fn totp_matches_rfc6238_sha1_vectors_truncated_to_6_digits() {
        // RFC 6238 附录 B（8 位）取后 6 位
        for (t, code8) in [
            (59u64, 94287082u32),
            (1111111109, 7081804),
            (2000000000, 69279037),
        ] {
            assert_eq!(hotp(RFC_SEED, step_at(t)), code8 % 1_000_000);
        }
    }

    #[test]
    fn verify_accepts_skew_and_rejects_replay_and_garbage() {
        let secret = base32_encode(RFC_SEED);
        let t = 1_111_111_109u64;
        let code = format!("{:06}", hotp(RFC_SEED, step_at(t)));
        assert_eq!(verify(&secret, &code, t, None), Some(step_at(t)));
        // 前后一个步长仍接受
        assert!(verify(&secret, &code, t + STEP_SECS, None).is_some());
        assert!(verify(&secret, &code, t - STEP_SECS, None).is_some());
        // 超出窗口拒绝
        assert!(verify(&secret, &code, t + 3 * STEP_SECS, None).is_none());
        // 防重放：同一步号已用过
        assert!(verify(&secret, &code, t, Some(step_at(t))).is_none());
        // 空格容忍、非法格式拒绝
        let spaced = format!("{} {}", &code[..3], &code[3..]);
        assert!(verify(&secret, &spaced, t, None).is_some());
        assert!(verify(&secret, "12345", t, None).is_none());
        assert!(verify(&secret, "abcdef", t, None).is_none());
        assert!(verify("", &code, t, None).is_none());
    }

    #[test]
    fn otpauth_uri_is_encoded() {
        let u = otpauth_uri("Flame Panel", "admin@x", "ABC");
        assert!(
            u.starts_with("otpauth://totp/Flame%20Panel:admin%40x?secret=ABC&issuer=Flame%20Panel")
        );
    }

    #[test]
    fn recovery_codes_are_unique_and_hash_normalized() {
        let codes = generate_recovery_codes();
        assert_eq!(codes.len(), RECOVERY_CODE_COUNT);
        let set: std::collections::HashSet<_> = codes.iter().collect();
        assert_eq!(set.len(), RECOVERY_CODE_COUNT);
        assert!(codes
            .iter()
            .all(|c| c.len() == 11 && c.as_bytes()[5] == b'-'));
        let c = &codes[0];
        assert_eq!(
            hash_recovery_code(c),
            hash_recovery_code(&c.to_lowercase().replace('-', " "))
        );
    }
}
