//! API 响应字段裁剪：`GET /api/...?fields=a,b,c`（Doc/30 HTTP 层 · `?fields=` 裁剪）。
//!
//! 通用中间件，任何返回 JSON 的 GET 接口都自动支持，处理器无需改动：
//! - 响应体是**数组** → 每个对象元素只保留所列字段；
//! - 响应体是**对象**且含 `items` / `data` / `list` / `entries` / `records` 数组（分页信封）→
//!   只裁剪这些数组的元素，`total` / `page` 等元数据原样保留；
//! - 其它对象（单资源详情）→ 裁剪对象本身的顶层字段。
//!
//! 约束：字段名 `[A-Za-z0-9_]{1,64}`、最多 32 个，非法 → 400；只处理 2xx + `application/json`
//! 且体积已知 ≤ [`MAX_TRIM_BODY_BYTES`] 的响应，其余原样透传。裁剪发生在鉴权之后（401 / 403
//! 不受影响），压缩由前置 nginx 完成，因此裁剪后的体积直接体现在传输量上。
use axum::{
    body::{Body, HttpBody},
    extract::Request,
    http::{header, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::{Map, Value};

use crate::core::error::AppError;

/// 最多可请求的字段数
pub const MAX_FIELDS: usize = 32;
/// 单个字段名最大长度
pub const MAX_FIELD_LEN: usize = 64;
/// 参与裁剪的响应体上限（更大的响应原样透传，避免整块缓冲）
pub const MAX_TRIM_BODY_BYTES: u64 = 16 * 1024 * 1024;
/// 分页信封中被视为"列表"的键
pub const LIST_KEYS: [&str; 5] = ["items", "data", "list", "entries", "records"];
/// 已裁剪响应的标记头
pub const FIELDS_APPLIED_HEADER: &str = "x-fields-applied";

/// 从原始 query 中取出 `fields` 并校验。`Ok(None)` = 未指定或为空。
pub fn parse_fields(query: Option<&str>) -> Result<Option<Vec<String>>, String> {
    let Some(query) = query.filter(|q| q.contains("fields")) else {
        return Ok(None);
    };
    let uri: axum::http::Uri = format!("/?{query}")
        .parse()
        .map_err(|_| "invalid query string".to_string())?;
    let pairs = axum::extract::Query::<Vec<(String, String)>>::try_from_uri(&uri)
        .map_err(|_| "invalid query string".to_string())?
        .0;
    let mut raw: Option<String> = None;
    for (k, v) in pairs {
        if k == "fields" {
            // 多次出现时合并：`?fields=a&fields=b` 等价于 `?fields=a,b`
            raw = Some(match raw {
                Some(prev) => format!("{prev},{v}"),
                None => v,
            });
        }
    }
    let Some(raw) = raw else {
        return Ok(None);
    };
    let mut fields: Vec<String> = Vec::new();
    for f in raw.split(',').map(str::trim).filter(|f| !f.is_empty()) {
        let valid =
            f.len() <= MAX_FIELD_LEN && f.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !valid {
            return Err(format!(
                "invalid field name '{}': expected [A-Za-z0-9_]{{1,{MAX_FIELD_LEN}}}",
                f.chars().take(MAX_FIELD_LEN).collect::<String>()
            ));
        }
        if !fields.iter().any(|x| x == f) {
            fields.push(f.to_string());
        }
    }
    if fields.len() > MAX_FIELDS {
        return Err(format!("too many fields: {} > {MAX_FIELDS}", fields.len()));
    }
    Ok((!fields.is_empty()).then_some(fields))
}

fn trim_object(obj: Map<String, Value>, fields: &[String]) -> Map<String, Value> {
    obj.into_iter()
        .filter(|(k, _)| fields.iter().any(|f| f == k))
        .collect()
}

fn trim_array(items: Vec<Value>, fields: &[String]) -> Vec<Value> {
    items
        .into_iter()
        .map(|v| match v {
            Value::Object(o) => Value::Object(trim_object(o, fields)),
            other => other,
        })
        .collect()
}

/// 按规则裁剪 JSON（见模块文档）。
pub fn select_fields(value: Value, fields: &[String]) -> Value {
    match value {
        Value::Array(items) => Value::Array(trim_array(items, fields)),
        Value::Object(obj) => {
            let is_envelope = LIST_KEYS
                .iter()
                .any(|k| matches!(obj.get(*k), Some(Value::Array(_))));
            if is_envelope {
                Value::Object(
                    obj.into_iter()
                        .map(|(k, v)| match v {
                            Value::Array(items) if LIST_KEYS.contains(&k.as_str()) => {
                                (k, Value::Array(trim_array(items, fields)))
                            }
                            other => (k, other),
                        })
                        .collect(),
                )
            } else {
                Value::Object(trim_object(obj, fields))
            }
        }
        other => other,
    }
}

fn is_json(res: &Response) -> bool {
    res.headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| {
            ct.trim_start()
                .to_ascii_lowercase()
                .starts_with("application/json")
        })
        .unwrap_or(false)
}

/// 中间件入口：挂在鉴权层之内（离处理器最近）。
pub async fn field_selection_middleware(req: Request, next: Next) -> Response {
    if req.method() != Method::GET || !req.uri().path().starts_with("/api/") {
        return next.run(req).await;
    }
    let fields = match parse_fields(req.uri().query()) {
        Ok(Some(f)) => f,
        Ok(None) => return next.run(req).await,
        Err(msg) => return AppError::BadRequest(msg).into_response(),
    };

    let res = next.run(req).await;
    if !res.status().is_success() || res.status() == StatusCode::NO_CONTENT || !is_json(&res) {
        return res;
    }
    match res.body().size_hint().upper() {
        Some(n) if n <= MAX_TRIM_BODY_BYTES => {}
        _ => return res,
    }

    let (mut parts, body) = res.into_parts();
    let bytes = match axum::body::to_bytes(body, MAX_TRIM_BODY_BYTES as usize).await {
        Ok(b) => b,
        Err(_) => {
            return AppError::internal("failed to buffer response for field selection")
                .into_response()
        }
    };
    let value: Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        // 声明 JSON 但解析失败：原样返回，不因裁剪功能破坏响应
        Err(_) => return Response::from_parts(parts, Body::from(bytes)),
    };
    let trimmed = match serde_json::to_vec(&select_fields(value, &fields)) {
        Ok(v) => v,
        Err(_) => return Response::from_parts(parts, Body::from(bytes)),
    };
    parts.headers.remove(header::CONTENT_LENGTH);
    // 内容已变化，原 ETag 不再成立
    parts.headers.remove(header::ETAG);
    parts
        .headers
        .insert(FIELDS_APPLIED_HEADER, HeaderValue::from_static("1"));
    Response::from_parts(parts, Body::from(trimmed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn f(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_fields_basic_and_dedup() {
        assert_eq!(parse_fields(None), Ok(None));
        assert_eq!(parse_fields(Some("page=1")), Ok(None));
        assert_eq!(parse_fields(Some("fields=")), Ok(None));
        assert_eq!(parse_fields(Some("fields=%20,%20")), Ok(None));
        assert_eq!(
            parse_fields(Some("page=2&fields=id,%20name,id")),
            Ok(Some(f(&["id", "name"])))
        );
        assert_eq!(
            parse_fields(Some("fields=id&fields=status")),
            Ok(Some(f(&["id", "status"])))
        );
    }

    #[test]
    fn parse_fields_rejects_bad_names_and_too_many() {
        assert!(parse_fields(Some("fields=a.b")).is_err());
        assert!(parse_fields(Some("fields=a-b")).is_err());
        assert!(parse_fields(Some(&format!("fields={}", "x".repeat(65)))).is_err());
        let many = (0..33)
            .map(|i| format!("f{i}"))
            .collect::<Vec<_>>()
            .join(",");
        assert!(parse_fields(Some(&format!("fields={many}"))).is_err());
        let ok = (0..32)
            .map(|i| format!("f{i}"))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(
            parse_fields(Some(&format!("fields={ok}"))).map(|v| v.map(|x| x.len())),
            Ok(Some(32))
        );
    }

    #[test]
    fn trims_top_level_array() {
        let v = json!([{"id": 1, "name": "a", "secret": 1}, {"id": 2}, 3]);
        assert_eq!(
            select_fields(v, &f(&["id", "name"])),
            json!([{"id": 1, "name": "a"}, {"id": 2}, 3])
        );
    }

    #[test]
    fn trims_envelope_lists_but_keeps_metadata() {
        let v = json!({"items": [{"id": 1, "x": 2}], "total": 10, "page": 1, "meta": {"id": 9, "y": 1}});
        assert_eq!(
            select_fields(v, &f(&["id"])),
            json!({"items": [{"id": 1}], "total": 10, "page": 1, "meta": {"id": 9, "y": 1}})
        );
        let v = json!({"data": [{"id": 1, "x": 2}], "entries": [{"x": 1}], "count": 1});
        assert_eq!(
            select_fields(v, &f(&["x"])),
            json!({"data": [{"x": 2}], "entries": [{"x": 1}], "count": 1})
        );
    }

    #[test]
    fn trims_single_object_without_list_keys() {
        let v = json!({"id": 1, "name": "a", "items": "not-an-array"});
        assert_eq!(select_fields(v, &f(&["name"])), json!({"name": "a"}));
    }

    #[test]
    fn scalars_pass_through() {
        assert_eq!(select_fields(json!(42), &f(&["id"])), json!(42));
        assert_eq!(select_fields(json!(null), &f(&["id"])), json!(null));
    }
}
