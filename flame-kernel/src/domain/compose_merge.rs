//! 应用升级的 compose 三方合并（Doc/30 M-C「应用升级：声明式可覆盖（三方合并）」）。
//!
//! 升级时有三份 compose：
//! - **base**：上次安装 / 升级时面板渲染并落盘的版本（`.flamepanel/compose.base.yml`）；
//! - **local**：当前磁盘上的 `docker-compose.yml`（用户可能手改过端口、卷、环境变量…）；
//! - **upstream**：目标版本模板按同一组参数渲染出的新版本。
//!
//! 旧实现直接用 upstream 覆盖 local，用户的手工修改在升级后静默丢失。这里按 YAML 结构
//! 做三方合并：
//! - 只有一方相对 base 改动 → 取改动方（用户改的保留，上游改的生效）；
//! - 两方改成同一个值 → 取该值；
//! - 两方都是映射 → 逐键递归（键顺序：local 原有顺序在前，上游新增键在后）；
//! - 其余（两方改成不同值、序列被两方各自修改、类型不同）→ **冲突**。
//!
//! 序列（`ports` / `volumes` / `command` …）整体视为一个值：逐元素合并在 compose 语义下
//! 没有可靠的对齐方式（端口映射去重、卷顺序都有语义）。键不存在用 `None` 表示，删除即
//! 「改成 None」，规则与修改一致。
//!
//! 文本快速路径：local 与 base 文本一致（用户未改）→ 原样取 upstream；upstream 与 base
//! 一致 → 原样保留 local。只有真正需要合并时才重新序列化（会丢失注释与格式，结果中
//! `reformatted = true` 提示调用方）。
use crate::core::error::AppError;
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};

/// 冲突处理策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(utoipa::ToSchema)]
pub enum ConflictStrategy {
    /// 有冲突即中止（默认）：调用方返回 409 并列出冲突，由用户选择后重试。
    #[default]
    Abort,
    /// 冲突处保留本地修改。
    Local,
    /// 冲突处采用上游新版本。
    Upstream,
}

impl ConflictStrategy {
    pub fn parse(s: &str) -> Result<Self, AppError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "" | "abort" => Ok(Self::Abort),
            "local" | "ours" => Ok(Self::Local),
            "upstream" | "theirs" => Ok(Self::Upstream),
            other => Err(AppError::BadRequest(format!(
                "未知冲突策略 {other:?}（可选 abort / local / upstream）"
            ))),
        }
    }
}

/// 单处冲突：路径 + 三方取值（YAML 片段，过长截断；`None` = 该键不存在）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct MergeConflict {
    pub path: String,
    pub base: Option<String>,
    pub local: Option<String>,
    pub upstream: Option<String>,
}

/// 合并结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct ComposeMerge {
    /// 合并后的 compose 文本（`Abort` 且有冲突时为保留本地值的草稿，不应落盘）。
    pub merged: String,
    pub conflicts: Vec<MergeConflict>,
    /// 保留下来的本地修改路径。
    pub local_changes: Vec<String>,
    /// 生效的上游修改路径。
    pub upstream_changes: Vec<String>,
    /// 是否经结构化重新序列化（注释 / 格式不保留）。
    pub reformatted: bool,
}

impl ComposeMerge {
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }
}

/// 升级预览（`GET /api/app-store/installed/{id}/upgrade-preview`）。
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct UpgradePreview {
    pub from_version: String,
    pub to_version: String,
    pub mode: String,
    /// 已是目标版本（无需升级）
    pub up_to_date: bool,
    /// 是否有安装时保存的 compose 基线；无基线时本地与新版本的每处差异都记为冲突
    pub base_available: bool,
    /// 容器模式的合并结果；原生 / WASM 模式为空
    pub merge: Option<ComposeMerge>,
}

/// 冲突 / 变更路径上限：防止异常大的 compose 让响应无限膨胀。
const MAX_REPORTED: usize = 200;
/// 冲突片段截断长度（字符）。
const SNIPPET_MAX: usize = 300;

fn parse(label: &str, text: &str) -> Result<Value, AppError> {
    let v: Value = serde_yaml::from_str(text)
        .map_err(|e| AppError::BadRequest(format!("{label} compose 不是合法 YAML: {e}")))?;
    match v {
        Value::Mapping(_) => Ok(v),
        // 空文件解析为 Null：视作空映射
        Value::Null => Ok(Value::Mapping(Mapping::new())),
        _ => Err(AppError::BadRequest(format!(
            "{label} compose 顶层必须是映射"
        ))),
    }
}

/// 三方合并入口。`base = None` 表示无基线（旧版本安装未保存）：此时无法区分用户修改，
/// 退化为「上游为准」，但所有与上游不同的本地值都记为冲突，交给策略决定。
pub fn three_way_merge(
    base: Option<&str>,
    local: &str,
    upstream: &str,
    strategy: ConflictStrategy,
) -> Result<ComposeMerge, AppError> {
    // 先校验两份输入：快速路径原样返回文本前也必须保证它是合法 compose（否则 up 必败）
    let local_v = parse("本地", local)?;
    let upstream_v = parse("新版本", upstream)?;
    if let Some(b) = base {
        if b == local {
            return Ok(ComposeMerge {
                merged: upstream.to_string(),
                conflicts: vec![],
                local_changes: vec![],
                upstream_changes: if b == upstream {
                    vec![]
                } else {
                    vec!["$".into()]
                },
                reformatted: false,
            });
        }
        if b == upstream {
            return Ok(ComposeMerge {
                merged: local.to_string(),
                conflicts: vec![],
                local_changes: vec!["$".into()],
                upstream_changes: vec![],
                reformatted: false,
            });
        }
    }
    if local_v == upstream_v {
        return Ok(ComposeMerge {
            merged: local.to_string(),
            conflicts: vec![],
            local_changes: vec![],
            upstream_changes: vec![],
            reformatted: false,
        });
    }
    let base_v = match base {
        Some(b) => Some(parse("基线", b)?),
        None => None,
    };

    let mut ctx = Ctx {
        strategy,
        conflicts: vec![],
        local_changes: vec![],
        upstream_changes: vec![],
    };
    let merged_v = match &base_v {
        Some(b) => ctx.merge("", Some(b), Some(&local_v), Some(&upstream_v)),
        // 无基线：以 upstream 为 base，local 的每处差异都是冲突
        None => ctx.merge_no_base("", &local_v, &upstream_v),
    }
    .unwrap_or_else(|| Value::Mapping(Mapping::new()));

    let merged = serde_yaml::to_string(&merged_v)
        .map_err(|e| AppError::internal(format!("序列化合并结果失败: {e}")))?;
    Ok(ComposeMerge {
        merged,
        conflicts: ctx.conflicts,
        local_changes: ctx.local_changes,
        upstream_changes: ctx.upstream_changes,
        reformatted: true,
    })
}

struct Ctx {
    strategy: ConflictStrategy,
    conflicts: Vec<MergeConflict>,
    local_changes: Vec<String>,
    upstream_changes: Vec<String>,
}

fn key_label(k: &Value) -> String {
    match k {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        other => serde_yaml::to_string(other)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

fn join(path: &str, k: &Value) -> String {
    let label = key_label(k);
    if path.is_empty() {
        label
    } else {
        format!("{path}.{label}")
    }
}

fn snippet(v: Option<&Value>) -> Option<String> {
    v.map(|v| {
        let s = serde_yaml::to_string(v).unwrap_or_default();
        let s = s.trim_end();
        if s.chars().count() > SNIPPET_MAX {
            let cut: String = s.chars().take(SNIPPET_MAX).collect();
            format!("{cut}…")
        } else {
            s.to_string()
        }
    })
}

fn push_capped(list: &mut Vec<String>, path: &str) {
    if list.len() < MAX_REPORTED {
        list.push(if path.is_empty() {
            "$".into()
        } else {
            path.into()
        });
    }
}

impl Ctx {
    /// 返回合并值；`None` = 该键在结果中不存在。
    fn merge(
        &mut self,
        path: &str,
        base: Option<&Value>,
        local: Option<&Value>,
        upstream: Option<&Value>,
    ) -> Option<Value> {
        if local == upstream {
            return local.cloned();
        }
        if local == base {
            push_capped(&mut self.upstream_changes, path);
            return upstream.cloned();
        }
        if upstream == base {
            push_capped(&mut self.local_changes, path);
            return local.cloned();
        }
        // 两方都是映射：逐键递归（基线缺失或非映射时按空映射对齐）
        if let (Some(Value::Mapping(l)), Some(Value::Mapping(u))) = (local, upstream) {
            let empty = Mapping::new();
            let b = match base {
                Some(Value::Mapping(b)) => b,
                _ => &empty,
            };
            return Some(Value::Mapping(self.merge_maps(path, b, l, u)));
        }
        self.conflict(path, base, local, upstream)
    }

    fn merge_maps(&mut self, path: &str, b: &Mapping, l: &Mapping, u: &Mapping) -> Mapping {
        let mut keys: Vec<&Value> = l.keys().collect();
        for k in u.keys().chain(b.keys()) {
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        let mut out = Mapping::new();
        for k in keys {
            let child = join(path, k);
            if let Some(v) = self.merge(&child, b.get(k), l.get(k), u.get(k)) {
                out.insert(k.clone(), v);
            }
        }
        out
    }

    fn merge_no_base(&mut self, path: &str, local: &Value, upstream: &Value) -> Option<Value> {
        if local == upstream {
            return Some(local.clone());
        }
        if let (Value::Mapping(l), Value::Mapping(u)) = (local, upstream) {
            let mut keys: Vec<&Value> = l.keys().collect();
            for k in u.keys() {
                if !keys.contains(&k) {
                    keys.push(k);
                }
            }
            let mut out = Mapping::new();
            for k in keys {
                let child = join(path, k);
                let merged = match (l.get(k), u.get(k)) {
                    (Some(lv), Some(uv)) => self.merge_no_base(&child, lv, uv),
                    // 仅上游有：新版本新增，直接采用
                    (None, Some(uv)) => {
                        push_capped(&mut self.upstream_changes, &child);
                        Some(uv.clone())
                    }
                    // 仅本地有：无法判断是用户新增还是上游删除 → 冲突
                    (Some(lv), None) => self.conflict(&child, None, Some(lv), None),
                    (None, None) => None,
                };
                if let Some(v) = merged {
                    out.insert(k.clone(), v);
                }
            }
            return Some(Value::Mapping(out));
        }
        self.conflict(path, None, Some(local), Some(upstream))
    }

    fn conflict(
        &mut self,
        path: &str,
        base: Option<&Value>,
        local: Option<&Value>,
        upstream: Option<&Value>,
    ) -> Option<Value> {
        if self.conflicts.len() < MAX_REPORTED {
            self.conflicts.push(MergeConflict {
                path: if path.is_empty() {
                    "$".into()
                } else {
                    path.into()
                },
                base: snippet(base),
                local: snippet(local),
                upstream: snippet(upstream),
            });
        }
        match self.strategy {
            ConflictStrategy::Upstream => upstream.cloned(),
            // Abort 也保留本地值：结果仅作预览草稿，调用方不会落盘
            ConflictStrategy::Local | ConflictStrategy::Abort => local.cloned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "services:\n  web:\n    image: nginx:1.25\n    ports:\n      - \"8080:80\"\n    environment:\n      TZ: UTC\n";

    fn yaml(s: &str) -> Value {
        serde_yaml::from_str(s).unwrap()
    }

    #[test]
    fn untouched_local_takes_upstream_verbatim() {
        let up = "# new comment\nservices:\n  web:\n    image: nginx:1.27\n";
        let m = three_way_merge(Some(BASE), BASE, up, ConflictStrategy::Abort).unwrap();
        assert_eq!(m.merged, up, "用户未改时应原样采用上游文本（保留注释）");
        assert!(!m.reformatted && !m.has_conflicts());
        assert_eq!(m.upstream_changes, vec!["$"]);
    }

    #[test]
    fn unchanged_upstream_keeps_local_verbatim() {
        let local = format!("{BASE}# my note\n");
        let m = three_way_merge(Some(BASE), &local, BASE, ConflictStrategy::Abort).unwrap();
        assert_eq!(m.merged, local);
        assert_eq!(m.local_changes, vec!["$"]);
    }

    #[test]
    fn disjoint_changes_merge_both_sides() {
        // 用户改端口 + 加环境变量；上游升级镜像 + 新增 healthcheck
        let local = BASE
            .replace("8080:80", "9000:80")
            .replace("TZ: UTC", "TZ: UTC\n      LANG: zh_CN.UTF-8");
        let up = BASE.replace("nginx:1.25", "nginx:1.27").replace(
            "    environment:",
            "    healthcheck:\n      test: [\"CMD\", \"true\"]\n    environment:",
        );
        let m = three_way_merge(Some(BASE), &local, &up, ConflictStrategy::Abort).unwrap();
        assert!(!m.has_conflicts(), "{:?}", m.conflicts);
        assert!(m.reformatted);
        let v = yaml(&m.merged);
        let web = &v["services"]["web"];
        assert_eq!(web["image"], yaml("nginx:1.27"));
        assert_eq!(web["ports"], yaml("[\"9000:80\"]"));
        assert_eq!(web["environment"]["LANG"], yaml("zh_CN.UTF-8"));
        assert!(web["healthcheck"].is_mapping());
        assert!(m.local_changes.contains(&"services.web.ports".to_string()));
        // 变更记录在最高的改动节点：上游没动 environment，整段算本地修改
        assert!(m
            .local_changes
            .contains(&"services.web.environment".to_string()));
        assert!(m
            .upstream_changes
            .contains(&"services.web.image".to_string()));
        assert!(m
            .upstream_changes
            .contains(&"services.web.healthcheck".to_string()));
    }

    #[test]
    fn same_value_on_both_sides_is_not_a_conflict() {
        let both = BASE.replace("nginx:1.25", "nginx:1.27");
        let local = format!("{both}x-note: mine\n");
        let m = three_way_merge(Some(BASE), &local, &both, ConflictStrategy::Abort).unwrap();
        assert!(!m.has_conflicts());
        assert_eq!(
            yaml(&m.merged)["services"]["web"]["image"],
            yaml("nginx:1.27")
        );
    }

    #[test]
    fn conflicting_scalar_respects_strategy() {
        let local = BASE.replace("nginx:1.25", "nginx:1.25-alpine");
        let up = BASE.replace("nginx:1.25", "nginx:1.27");
        let abort = three_way_merge(Some(BASE), &local, &up, ConflictStrategy::Abort).unwrap();
        assert_eq!(abort.conflicts.len(), 1);
        let c = &abort.conflicts[0];
        assert_eq!(c.path, "services.web.image");
        assert_eq!(c.base.as_deref(), Some("nginx:1.25"));
        assert_eq!(c.local.as_deref(), Some("nginx:1.25-alpine"));
        assert_eq!(c.upstream.as_deref(), Some("nginx:1.27"));

        let keep = three_way_merge(Some(BASE), &local, &up, ConflictStrategy::Local).unwrap();
        assert_eq!(
            yaml(&keep.merged)["services"]["web"]["image"],
            yaml("nginx:1.25-alpine")
        );
        let take = three_way_merge(Some(BASE), &local, &up, ConflictStrategy::Upstream).unwrap();
        assert_eq!(
            yaml(&take.merged)["services"]["web"]["image"],
            yaml("nginx:1.27")
        );
    }

    #[test]
    fn sequences_are_atomic() {
        let local = BASE.replace("- \"8080:80\"", "- \"8080:80\"\n      - \"8443:443\"");
        let up = BASE.replace("- \"8080:80\"", "- \"8080:80\"\n      - \"9090:90\"");
        let m = three_way_merge(Some(BASE), &local, &up, ConflictStrategy::Abort).unwrap();
        assert_eq!(m.conflicts.len(), 1);
        assert_eq!(m.conflicts[0].path, "services.web.ports");
    }

    #[test]
    fn deletions_follow_the_same_rules() {
        // 用户删掉 environment；上游没动它 → 保持删除；上游删掉 ports，用户没动 → 删除
        let local = "services:\n  web:\n    image: nginx:1.25\n    ports:\n      - \"8080:80\"\n";
        let up = "services:\n  web:\n    image: nginx:1.25\n    environment:\n      TZ: UTC\n";
        let m = three_way_merge(Some(BASE), local, up, ConflictStrategy::Abort).unwrap();
        assert!(!m.has_conflicts(), "{:?}", m.conflicts);
        let web = &yaml(&m.merged)["services"]["web"];
        assert!(web.get("environment").is_none());
        assert!(web.get("ports").is_none());

        // 用户改了 environment、上游删了它 → 冲突（修改 vs 删除）
        let local2 = BASE.replace("TZ: UTC", "TZ: Asia/Shanghai");
        let m2 = three_way_merge(
            Some(BASE),
            &local2,
            up.replace("    environment:\n      TZ: UTC\n", "").as_str(),
            ConflictStrategy::Abort,
        )
        .unwrap();
        assert!(m2
            .conflicts
            .iter()
            .any(|c| c.path == "services.web.environment" && c.upstream.is_none()));
    }

    #[test]
    fn missing_base_flags_local_differences() {
        let local = BASE.replace("8080:80", "9000:80");
        let up = BASE
            .replace("nginx:1.25", "nginx:1.27")
            .replace("8080:80", "8081:80");
        let m = three_way_merge(None, &local, &up, ConflictStrategy::Abort).unwrap();
        let paths: Vec<&str> = m.conflicts.iter().map(|c| c.path.as_str()).collect();
        assert!(paths.contains(&"services.web.image"));
        assert!(paths.contains(&"services.web.ports"));
        let take = three_way_merge(None, &local, &up, ConflictStrategy::Upstream).unwrap();
        assert_eq!(yaml(&take.merged), yaml(&up));
        // 本地与上游一致：无冲突
        let same = three_way_merge(None, &up, &up, ConflictStrategy::Abort).unwrap();
        assert!(!same.has_conflicts() && !same.reformatted);
    }

    #[test]
    fn rejects_invalid_yaml_and_non_mapping_top_level() {
        assert!(three_way_merge(Some(BASE), "a: [", BASE, ConflictStrategy::Abort).is_err());
        assert!(three_way_merge(Some(BASE), BASE, "a: [", ConflictStrategy::Abort).is_err());
        assert!(
            three_way_merge(Some(BASE), "- a\n- b\n", "x: 1\n", ConflictStrategy::Abort).is_err()
        );
    }

    #[test]
    fn strategy_parse() {
        assert_eq!(
            ConflictStrategy::parse("").unwrap(),
            ConflictStrategy::Abort
        );
        assert_eq!(
            ConflictStrategy::parse("Local").unwrap(),
            ConflictStrategy::Local
        );
        assert_eq!(
            ConflictStrategy::parse("theirs").unwrap(),
            ConflictStrategy::Upstream
        );
        assert!(ConflictStrategy::parse("merge-all").is_err());
    }
}
