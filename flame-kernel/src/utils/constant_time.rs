/// 常量时间字符串/字节序列比较，避免基于比较时长的时序侧信道（长度早退除外——
/// 长度差异本身可被观测，但长度通常是公开信息；核心是逐字节异或聚合，不短路）。
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for i in 0..a.len() {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

/// M4：对 refresh token 做 SHA-256 哈希，用于黑名单/吊销比对。
/// 黑名单只存哈希而非明文，避免内存中残留可用的原始凭据。
pub fn constant_time_eq_hash(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex_encode(&hasher.finalize())
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    // Shared data only: this module is absent from non-test builds.
    mod constant_time_eq_vectors {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../test-fixtures/security/constant_time_eq.rs"
        ));
    }

    #[test]
    fn constant_time_eq_shared_utf8_vectors() {
        for &(name, left, right, equal) in constant_time_eq_vectors::UTF8_CASES {
            assert_eq!(
                constant_time_eq(left.as_bytes(), right.as_bytes()),
                equal,
                "case: {name}"
            );
            assert_eq!(
                constant_time_eq(right.as_bytes(), left.as_bytes()),
                equal,
                "reverse case: {name}"
            );
            assert!(
                constant_time_eq(left.as_bytes(), left.as_bytes()),
                "reflexive case: {name}"
            );
        }
    }

    #[test]
    fn constant_time_eq_every_raw_byte_at_every_position() {
        let expected = [0xa5; 64];
        for position in 0..expected.len() {
            let mut candidate = expected;
            for replacement in 0u8..=u8::MAX {
                candidate[position] = replacement;
                let equal = replacement == 0xa5;
                assert_eq!(
                    constant_time_eq(&expected, &candidate),
                    equal,
                    "position: {position}, byte: {replacement}"
                );
                assert_eq!(
                    constant_time_eq(&candidate, &expected),
                    equal,
                    "reverse position: {position}, byte: {replacement}"
                );
            }
        }
        for length in 0..expected.len() {
            assert!(!constant_time_eq(&expected[..length], &expected));
            assert!(!constant_time_eq(&expected, &expected[..length]));
        }
    }

    #[test]
    fn constant_time_eq_works() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
        assert!(constant_time_eq(b"", b""));
    }
}
