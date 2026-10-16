//! PHP 运行时安装规格（M-B 主线二 Stage A-2 · Doc/20 §5.3）。
//!
//! 安装来源优先级：发行版包（php-fpm + 常用扩展）→ Sury/Remi 仓库（多版本）。
//! Stage A-2 先落发行版包路径；Sury/Remi 仓库引导由后续 Stage 补齐
//! （复用 `webserver/capability.rs` 的 lsphp 仓库探测经验）。

use super::LanguageRuntimeSpec;

/// PHP 版本 → 系统包名映射（声明式，Doc/20 §5.3 风险表 3：发行版碎片化集中管理）。
///
/// Debian/Ubuntu 多版本走 Sury 命名（php8.2-fpm）；RHEL 系由 PackageManager
/// 按包名路由（Remi 场景包名同为 php-fpm 前缀，仓库引导后续 Stage 补齐）。
pub fn spec() -> LanguageRuntimeSpec {
    LanguageRuntimeSpec {
        kind: "php",
        packages: &[
            ("8.1", "php8.1-fpm"),
            ("8.2", "php8.2-fpm"),
            ("8.3", "php8.3-fpm"),
        ],
        fallback: |v| format!("php{}-fpm", v),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_declares_php_kind_and_versions() {
        let s = spec();
        assert_eq!(s.kind, "php");
        assert!(s.packages.iter().any(|(v, _)| *v == "8.2"));
        assert_eq!((s.fallback)("8.4"), "php8.4-fpm");
    }
}
