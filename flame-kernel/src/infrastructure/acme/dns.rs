//! DNS Provider 端口（M-B 主线三 §6.1 · Stage 2）。
//!
//! DNS-01 验证需要把 TXT 记录写入域名 DNS。真实厂商 API（Cloudflare /
//! 阿里云 / 腾讯云 / 华为云）接入属后续增量——本模块先落端口（六边形）+
//! 内存实现（测试），签发编排只依赖端口不感知厂商差异。

use crate::core::error::AppError;
use crate::utils::sync::lock_or_recover;
use async_trait::async_trait;

/// DNS-01 记录操作端口。
#[async_trait]
pub trait DnsProvider: Send + Sync {
    /// 厂商标识（cloudflare | aliyun | tencent | huawei | memory），对齐
    /// `Certificate::is_valid_dns_provider` 白名单。
    fn name(&self) -> &'static str;

    /// 写入 TXT 记录（ACME DNS-01：`_acme-challenge.<domain>` → txt_value）。
    /// 实现须保证幂等（重复写入同值不报错）。
    async fn set_txt_record(&self, domain: &str, txt_value: &str) -> Result<(), AppError>;

    /// 清理 TXT 记录（签发成功/失败后调用；缺失不报错）。
    async fn clear_txt_record(&self, domain: &str, txt_value: &str) -> Result<(), AppError>;
}

/// 内存 DNS Provider（测试与无厂商场景）。
#[derive(Default)]
pub struct InMemoryDnsProvider {
    records: std::sync::Mutex<std::collections::HashMap<String, Vec<String>>>,
}

impl InMemoryDnsProvider {
    pub fn new() -> Self {
        Self::default()
    }

    /// 当前全部记录（测试断言用）。
    pub fn snapshot(&self) -> std::collections::HashMap<String, Vec<String>> {
        lock_or_recover(&self.records).clone()
    }
}

#[async_trait]
impl DnsProvider for InMemoryDnsProvider {
    fn name(&self) -> &'static str {
        "memory"
    }

    async fn set_txt_record(&self, domain: &str, txt_value: &str) -> Result<(), AppError> {
        let mut records = lock_or_recover(&self.records);
        let entry = records.entry(domain.to_string()).or_default();
        if !entry.iter().any(|v| v == txt_value) {
            entry.push(txt_value.to_string());
        }
        Ok(())
    }

    async fn clear_txt_record(&self, domain: &str, txt_value: &str) -> Result<(), AppError> {
        let mut records = lock_or_recover(&self.records);
        if let Some(entry) = records.get_mut(domain) {
            entry.retain(|v| v != txt_value);
            if entry.is_empty() {
                records.remove(domain);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn set_and_clear_txt_idempotent() {
        let p = InMemoryDnsProvider::new();
        p.set_txt_record("_acme-challenge.example.com", "abc")
            .await
            .unwrap();
        // 幂等：重复写同值不重复
        p.set_txt_record("_acme-challenge.example.com", "abc")
            .await
            .unwrap();
        let snap = p.snapshot();
        assert_eq!(
            snap.get("_acme-challenge.example.com").unwrap(),
            &vec!["abc".to_string()]
        );
        // 清理缺失值不报错
        p.clear_txt_record("_acme-challenge.example.com", "nope")
            .await
            .unwrap();
        p.clear_txt_record("_acme-challenge.example.com", "abc")
            .await
            .unwrap();
        assert!(p.snapshot().is_empty());
        // 再清一次（键不存在）仍幂等
        p.clear_txt_record("_acme-challenge.example.com", "abc")
            .await
            .unwrap();
    }
}
