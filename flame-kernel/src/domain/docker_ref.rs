//! Docker 对象引用的值校验（纯函数，领域规则；应用层与适配层共用）。

use crate::core::error::AppError;

/// Docker 对象引用（容器 / 网络 / 卷的 ID 或名称）白名单。
///
/// Doc/33 P-12：bollard 把 ID 原样拼进 Engine API 路径（`/containers/{id}/stop`）再经
/// `Url::join` 规范化——`..` 段会被折叠、`?` 开启查询串。axum 的 `Path` 会解码 `%2F`，
/// 因此 `x%2F..%2F..%2Fimages%2Fprune%3F` 能把「停止容器」变成 `POST /images/prune`，
/// 仅持 `docker:stop` 即可调用任意 POST 端点。与 Docker 自身的名称规则一致：
/// `[A-Za-z0-9][A-Za-z0-9_.-]*`，≤ 255。
pub fn validate_docker_ref(kind: &str, value: &str) -> Result<(), AppError> {
    let ok = !value.is_empty()
        && value.len() <= 255
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'));
    if ok {
        Ok(())
    } else {
        Err(AppError::ValidationError(format!(
            "{kind}: must match [A-Za-z0-9][A-Za-z0-9_.-]* (max 255), got {value:?}"
        )))
    }
}

/// 镜像引用（`registry:5000/ns/name:tag@sha256:…` 或镜像 ID）。允许 `/ : @`，
/// 但拒绝 `..`、空段、`?` / `#` / 空白等会改变 Engine API 路径语义的字符。
pub fn validate_image_ref(kind: &str, value: &str) -> Result<(), AppError> {
    let ok = !value.is_empty()
        && value.len() <= 512
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && !value.contains("..")
        && !value.contains("//")
        && !value.ends_with('/')
        && value.bytes().all(|b| {
            b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-' | b'/' | b':' | b'@')
        });
    if ok {
        Ok(())
    } else {
        Err(AppError::ValidationError(format!(
            "{kind}: invalid image reference {value:?}"
        )))
    }
}

/// 镜像 tag：`[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}`（Docker 规范）。
pub fn validate_image_tag(value: &str) -> Result<(), AppError> {
    let ok = !value.is_empty()
        && value.len() <= 128
        && (value.as_bytes()[0].is_ascii_alphanumeric() || value.starts_with('_'))
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'));
    if ok {
        Ok(())
    } else {
        Err(AppError::ValidationError(format!(
            "tag: must match [A-Za-z0-9_][A-Za-z0-9_.-]{{0,127}}, got {value:?}"
        )))
    }
}

/// 网络 / 卷驱动名：小写字母数字与 `_ . -`（`bridge`、`overlay`、`local`、插件如 `rexray.ebs`）。
pub fn validate_driver(value: &str) -> Result<(), AppError> {
    let ok = !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|b| {
            b.is_ascii_lowercase()
                || b.is_ascii_digit()
                || matches!(b, b'_' | b'.' | b'-' | b'/' | b':')
        })
        && !value.contains("..");
    if ok {
        Ok(())
    } else {
        Err(AppError::ValidationError(format!(
            "driver: invalid driver name {value:?}"
        )))
    }
}

/// CIDR 子网（IPv4 `a.b.c.d/0-32` 或 IPv6 `…/0-128`）。
pub fn validate_subnet(value: &str) -> Result<(), AppError> {
    let parsed = value.split_once('/').and_then(|(ip, len)| {
        let len: u8 = len.parse().ok()?;
        match ip.parse::<std::net::IpAddr>().ok()? {
            std::net::IpAddr::V4(_) if len <= 32 => Some(()),
            std::net::IpAddr::V6(_) if len <= 128 => Some(()),
            _ => None,
        }
    });
    parsed.ok_or_else(|| {
        AppError::ValidationError(format!(
            "subnet: expected CIDR like 172.20.0.0/16, got {value:?}"
        ))
    })
}

/// Compose 项目名：与 `infrastructure::naming::sanitize_project_name` 同规则
/// （首字符字母数字、`[A-Za-z0-9_.-]`、≤ 63、不含 `..`），此前 up / down 未校验。
pub fn validate_project_name(value: &str) -> Result<(), AppError> {
    let ok = !value.is_empty()
        && value.len() <= 63
        && !value.contains("..")
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'));
    if ok {
        Ok(())
    } else {
        Err(AppError::ValidationError(format!(
            "project_name: must match [A-Za-z0-9][A-Za-z0-9_.-]* (max 63, no '..'), got {value:?}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docker_refs() {
        for ok in ["abc123", "c1", "test-net", "my_vol.1", &"f".repeat(64)] {
            assert!(validate_docker_ref("id", ok).is_ok(), "{ok}");
        }
        for bad in [
            "",
            "x/../../images/prune",
            "../images/prune?",
            "a?b",
            "a#b",
            "-rf",
            ".hidden",
            "/abs",
            "a b",
            "a%2Fb",
            &"a".repeat(256),
        ] {
            assert!(validate_docker_ref("id", bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn image_refs_and_tags() {
        for ok in [
            "nginx",
            "nginx:1.27",
            "sha256:abc",
            "docker.io/library/busybox:1.36",
            "registry.local:5000/team/app@sha256:0123abcd",
        ] {
            assert!(validate_image_ref("image", ok).is_ok(), "{ok}");
        }
        for bad in [
            "",
            "../x",
            "a/../../b",
            "a//b",
            "a/",
            "/a",
            "a?b",
            "a b",
            "-x",
            "a#b",
        ] {
            assert!(validate_image_ref("image", bad).is_err(), "{bad}");
        }
        assert!(validate_image_tag("1.0").is_ok());
        assert!(validate_image_tag("_v").is_ok());
        assert!(validate_image_tag("latest").is_ok());
        assert!(validate_image_tag("").is_err());
        assert!(validate_image_tag("a/b").is_err());
        assert!(validate_image_tag(".x").is_err());
        assert!(validate_image_tag(&"a".repeat(129)).is_err());
    }

    #[test]
    fn drivers_and_subnets() {
        for ok in [
            "bridge",
            "overlay",
            "local",
            "rexray.ebs",
            "vieux/sshfs:latest",
        ] {
            assert!(validate_driver(ok).is_ok(), "{ok}");
        }
        for bad in ["", "Bridge", "a b", "x?y", "../x"] {
            assert!(validate_driver(bad).is_err(), "{bad}");
        }
        for ok in ["172.20.0.0/16", "10.0.0.0/8", "fd00::/64"] {
            assert!(validate_subnet(ok).is_ok(), "{ok}");
        }
        for bad in ["172.20.0.0", "172.20.0.0/33", "abc/16", "fd00::/129", ""] {
            assert!(validate_subnet(bad).is_err(), "{bad}");
        }
    }
}
