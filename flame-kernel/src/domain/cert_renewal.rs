//! 证书续签历史（Doc/30 S-10）：把 Outbox 中的证书领域事件归并成面板可读的时间线。
//!
//! 数据来源是既有的事件落库（`outbox_events`，每条 `DomainEvent` 都会以
//! 外部标签 JSON 形式存档，如 `{"CertificateRenewed":{"id":3,"domain":"a.com"}}`），
//! 因此**不新增表、不改续签扫描**——续签历史天然覆盖历史数据。
//!
//! 本模块只含纯函数：
//! - [`parse_event`]：单条落库事件 → 指定证书的时间线条目（不相关 / 损坏载荷返回 None）；
//! - [`build_timeline`]：排序、合并「自动续签前的签发事件」、截断。
//!
//! 合并规则：自动续签内部复用签发链路，会先发 `CertificateIssued` 再发
//! `CertificateRenewed`；两者相隔不超过 [`AUTO_RENEW_MERGE_WINDOW_SECS`] 秒时视为同一次
//! 自动续签，只保留 `auto_renewed`，避免时间线上每次续签出现两行。

use crate::domain::entity::OutboxEvent;
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

/// 参与续签历史的事件类型（Outbox `event_type` 列取值）。
pub const HISTORY_EVENT_TYPES: [&str; 4] = [
    "CertificateIssued",
    "CertificateRenewed",
    "CertificateRenewalFailed",
    "CertificateExpiring",
];

/// 签发事件与紧随其后的自动续签事件的合并窗口（秒）。
pub const AUTO_RENEW_MERGE_WINDOW_SECS: i64 = 300;

/// 时间线默认 / 最大条数。
pub const DEFAULT_HISTORY_LIMIT: usize = 50;
pub const MAX_HISTORY_LIMIT: usize = 200;

/// 时间线条目种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RenewalEventKind {
    /// 签发 / 手动续签成功（含首次签发）
    Issued,
    /// 后台扫描自动续签成功
    AutoRenewed,
    /// 后台扫描自动续签失败（`error` 为失败原因）
    AutoRenewFailed,
    /// 进入 30 / 7 天到期预警
    ExpiryWarning,
}

/// 续签时间线的一条记录。
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct RenewalEntry {
    /// 对应的 Outbox 事件 ID（稳定排序键，前端列表 key）
    pub event_id: i64,
    pub kind: RenewalEventKind,
    /// 事件记录的主域名
    pub domain: String,
    /// 失败原因（仅 `auto_renew_failed`）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// 预警事件记录的到期时刻（RFC 3339，仅 `expiry_warning`）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    /// 签发来源（仅 `issued`：letsencrypt / zerossl / self_signed / uploaded）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub at: DateTime<Utc>,
}

fn str_field(obj: &serde_json::Value, key: &str) -> Option<String> {
    obj.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

/// 单条 Outbox 事件 → 指定证书的时间线条目。
///
/// 事件类型不在 [`HISTORY_EVENT_TYPES`]、载荷损坏、或证书 ID 不匹配时返回 None
/// （损坏的历史记录不应让整条时间线失败）。
pub fn parse_event(ev: &OutboxEvent, cert_id: i64) -> Option<RenewalEntry> {
    let payload: serde_json::Value = serde_json::from_str(&ev.payload).ok()?;
    let body = payload.get(ev.event_type.as_str())?;
    let (kind, id_key, domain_key) = match ev.event_type.as_str() {
        "CertificateIssued" => (RenewalEventKind::Issued, "cert_id", "domains"),
        "CertificateRenewed" => (RenewalEventKind::AutoRenewed, "id", "domain"),
        "CertificateRenewalFailed" => (RenewalEventKind::AutoRenewFailed, "id", "domain"),
        "CertificateExpiring" => (RenewalEventKind::ExpiryWarning, "cert_id", "domains"),
        _ => return None,
    };
    if body.get(id_key)?.as_i64()? != cert_id {
        return None;
    }
    Some(RenewalEntry {
        event_id: ev.id,
        kind,
        domain: str_field(body, domain_key).unwrap_or_default(),
        error: (kind == RenewalEventKind::AutoRenewFailed)
            .then(|| str_field(body, "error"))
            .flatten(),
        expires_at: (kind == RenewalEventKind::ExpiryWarning)
            .then(|| str_field(body, "expires_at"))
            .flatten(),
        source: (kind == RenewalEventKind::Issued)
            .then(|| str_field(body, "source"))
            .flatten(),
        at: ev.created_at,
    })
}

/// 归并时间线：解析 → 按事件 ID 升序 → 合并自动续签前的签发事件 → 倒序截断到 `limit`。
///
/// 返回 `(entries, truncated)`；`truncated = true` 表示还有更早的记录未返回。
pub fn build_timeline(
    events: &[OutboxEvent],
    cert_id: i64,
    limit: usize,
) -> (Vec<RenewalEntry>, bool) {
    let mut parsed: Vec<RenewalEntry> = events
        .iter()
        .filter_map(|e| parse_event(e, cert_id))
        .collect();
    parsed.sort_by_key(|e| e.event_id);
    parsed.dedup_by_key(|e| e.event_id);

    let mut merged: Vec<RenewalEntry> = Vec::with_capacity(parsed.len());
    for entry in parsed {
        if entry.kind == RenewalEventKind::AutoRenewed {
            if let Some(last) = merged.last() {
                let gap = (entry.at - last.at).num_seconds();
                if last.kind == RenewalEventKind::Issued
                    && (0..=AUTO_RENEW_MERGE_WINDOW_SECS).contains(&gap)
                {
                    merged.pop();
                }
            }
        }
        merged.push(entry);
    }
    merged.reverse();
    let limit = limit.clamp(1, MAX_HISTORY_LIMIT);
    let truncated = merged.len() > limit;
    merged.truncate(limit);
    (merged, truncated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ev(id: i64, event_type: &str, payload: serde_json::Value, secs: i64) -> OutboxEvent {
        OutboxEvent {
            id,
            event_type: event_type.to_string(),
            payload: payload.to_string(),
            published: true,
            created_at: Utc.timestamp_opt(1_790_000_000 + secs, 0).unwrap(),
        }
    }

    fn issued(id: i64, cert: i64, secs: i64) -> OutboxEvent {
        ev(
            id,
            "CertificateIssued",
            serde_json::json!({"CertificateIssued": {"cert_id": cert, "domains": "a.test", "source": "letsencrypt"}}),
            secs,
        )
    }

    fn renewed(id: i64, cert: i64, secs: i64) -> OutboxEvent {
        ev(
            id,
            "CertificateRenewed",
            serde_json::json!({"CertificateRenewed": {"id": cert, "domain": "a.test"}}),
            secs,
        )
    }

    #[test]
    fn parses_each_kind_and_filters_by_certificate() {
        let failed = ev(
            3,
            "CertificateRenewalFailed",
            serde_json::json!({"CertificateRenewalFailed": {"id": 7, "domain": "a.test", "error": "HTTP-01 timeout"}}),
            0,
        );
        let e = parse_event(&failed, 7).unwrap();
        assert_eq!(e.kind, RenewalEventKind::AutoRenewFailed);
        assert_eq!(e.error.as_deref(), Some("HTTP-01 timeout"));
        assert!(
            parse_event(&failed, 8).is_none(),
            "其他证书的事件不入时间线"
        );

        let warn = ev(
            4,
            "CertificateExpiring",
            serde_json::json!({"CertificateExpiring": {"cert_id": 7, "domains": "a.test", "expires_at": "2026-10-01T00:00:00Z"}}),
            0,
        );
        let e = parse_event(&warn, 7).unwrap();
        assert_eq!(e.kind, RenewalEventKind::ExpiryWarning);
        assert_eq!(e.expires_at.as_deref(), Some("2026-10-01T00:00:00Z"));
        assert!(e.error.is_none());

        let e = parse_event(&issued(5, 7, 0), 7).unwrap();
        assert_eq!(e.kind, RenewalEventKind::Issued);
        assert_eq!(e.source.as_deref(), Some("letsencrypt"));
        assert_eq!(e.domain, "a.test");
    }

    #[test]
    fn ignores_foreign_types_and_corrupt_payloads() {
        let other = ev(
            1,
            "UserLoggedIn",
            serde_json::json!({"UserLoggedIn": {"id": 7}}),
            0,
        );
        assert!(parse_event(&other, 7).is_none());
        let mut broken = renewed(2, 7, 0);
        broken.payload = "{not json".into();
        assert!(parse_event(&broken, 7).is_none());
        // 标签与 event_type 不一致 / 缺 id 字段
        let mismatched = ev(
            3,
            "CertificateRenewed",
            serde_json::json!({"Other": {"id": 7}}),
            0,
        );
        assert!(parse_event(&mismatched, 7).is_none());
        let no_id = ev(
            4,
            "CertificateRenewed",
            serde_json::json!({"CertificateRenewed": {"domain": "a"}}),
            0,
        );
        assert!(parse_event(&no_id, 7).is_none());
    }

    #[test]
    fn merges_issue_event_emitted_by_auto_renewal() {
        // 首次签发（远早于续签）保留；自动续签内部的签发事件（紧邻 5 秒）被合并
        let events = vec![
            issued(1, 7, 0),
            issued(10, 7, 86_400 * 60),
            renewed(11, 7, 86_400 * 60 + 5),
        ];
        let (entries, truncated) = build_timeline(&events, 7, 50);
        assert!(!truncated);
        let kinds: Vec<_> = entries.iter().map(|e| e.kind).collect();
        assert_eq!(
            kinds,
            vec![RenewalEventKind::AutoRenewed, RenewalEventKind::Issued]
        );
        assert_eq!(entries[0].event_id, 11, "最新在前");
        assert_eq!(entries[1].event_id, 1);
    }

    #[test]
    fn manual_issue_far_before_renewal_is_not_merged() {
        let events = vec![
            issued(1, 7, 0),
            renewed(2, 7, AUTO_RENEW_MERGE_WINDOW_SECS + 1),
        ];
        let (entries, _) = build_timeline(&events, 7, 50);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn sorts_by_event_id_dedups_and_truncates() {
        // 输入乱序且含重复（多类型分页查询可能重叠）
        let events = vec![
            renewed(3, 7, 30),
            issued(1, 7, 0),
            renewed(3, 7, 30),
            renewed(2, 7, 1_000),
        ];
        let (entries, truncated) = build_timeline(&events, 7, 50);
        assert_eq!(
            entries.iter().map(|e| e.event_id).collect::<Vec<_>>(),
            vec![3, 2, 1]
        );
        assert!(!truncated);
        let (entries, truncated) = build_timeline(&events, 7, 2);
        assert_eq!(
            entries.iter().map(|e| e.event_id).collect::<Vec<_>>(),
            vec![3, 2]
        );
        assert!(truncated);
        // limit 下限 1、上限 MAX_HISTORY_LIMIT
        assert_eq!(build_timeline(&events, 7, 0).0.len(), 1);
    }
}
