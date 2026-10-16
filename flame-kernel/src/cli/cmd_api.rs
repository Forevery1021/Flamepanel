//! 在线类命令：api / openapi / backup list|create。

use std::io::Read;

use serde_json::{json, Value};

use super::args::Parsed;
use super::context::Ctx;
use super::http::{into_result, sign_token, Client};
use super::output::{CliError, Outcome};
use super::system::{human_bytes, table};

const METHODS: [&str; 6] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"];

/// `--data`：内联 JSON、`@文件`（相对原工作目录）、`-`（stdin）。
fn read_body(ctx: &Ctx, raw: &str) -> Result<Value, CliError> {
    let text = if raw == "-" {
        let mut s = String::new();
        std::io::stdin()
            .take(16 * 1024 * 1024)
            .read_to_string(&mut s)
            .map_err(|e| CliError::io("read body from stdin", &e))?;
        s
    } else if let Some(f) = raw.strip_prefix('@') {
        let path = ctx.user_path(f);
        std::fs::read_to_string(&path).map_err(|e| CliError::io(path.display(), &e))?
    } else {
        raw.to_string()
    };
    serde_json::from_str(&text)
        .map_err(|e| CliError::usage(format!("--data is not valid JSON: {e}")))
}

pub async fn api(ctx: &Ctx, p: &Parsed) -> Result<Outcome, CliError> {
    ctx.require_install()?;
    let method = p.args[0].to_ascii_uppercase();
    if !METHODS.contains(&method.as_str()) {
        return Err(CliError::usage(format!(
            "unsupported method '{}', expected one of {}",
            p.args[0],
            METHODS.join(", ")
        )));
    }
    let path = &p.args[1];
    if !path.starts_with('/') {
        return Err(CliError::usage(format!("path must start with '/': {path}")));
    }
    let body = p.value("data").map(|d| read_body(ctx, d)).transpose()?;
    let client = Client::new(ctx)?;

    if p.dry_run() && method != "GET" && method != "HEAD" {
        let req = json!({
            "method": method,
            "url": format!("{}{path}", client.base),
            "user": p.value("user"),
            "body": body,
        });
        return Ok(Outcome::new(
            json!({ "dry_run": true, "request": req }),
            format!(
                "would {method} {}{path}{}",
                client.base,
                body.as_ref().map(|b| format!(" {b}")).unwrap_or_default()
            ),
        ));
    }

    let (token, user) = sign_token(ctx, p.value("user")).await?;
    let resp = client.request(&method, path, Some(&token), body).await?;
    let status = resp.status;
    let rid = resp.request_id.clone();
    let v = into_result(resp)?;
    let human = match &v {
        Value::Null => format!("HTTP {status} (empty body)\n"),
        Value::String(s) => format!("{s}\n"),
        other => format!(
            "{}\n",
            serde_json::to_string_pretty(other).unwrap_or_default()
        ),
    };
    // data 即 API 响应体（不混入 CLI 元信息）；人类模式在 stderr 注明身份与请求编号
    if !p.json() {
        eprintln!(
            "HTTP {status} as '{user}'{}",
            rid.map(|r| format!(" · request {r}")).unwrap_or_default()
        );
    }
    Ok(Outcome::new(v, human))
}

pub async fn openapi() -> Result<Outcome, CliError> {
    let doc = crate::openapi::openapi_json();
    let human = format!(
        "{}\n",
        serde_json::to_string_pretty(&doc).unwrap_or_default()
    );
    Ok(Outcome::new(doc, human))
}

pub async fn backup_list(ctx: &Ctx) -> Result<Outcome, CliError> {
    ctx.require_install()?;
    let (token, _) = sign_token(ctx, None).await?;
    let v = into_result(
        Client::new(ctx)?
            .request("GET", "/api/backups", Some(&token), None)
            .await?,
    )?;
    let items = v.as_array().cloned().unwrap_or_default();
    let rows: Vec<Vec<String>> = items
        .iter()
        .map(|b| {
            vec![
                b.get("filename")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                b.get("size")
                    .and_then(Value::as_u64)
                    .map(human_bytes)
                    .unwrap_or_default(),
                b.get("created_at")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            ]
        })
        .collect();
    let human = if rows.is_empty() {
        "(no backups)\n".into()
    } else {
        table(&["FILENAME", "SIZE", "CREATED"], &rows)
    };
    Ok(Outcome::new(json!({ "backups": items }), human))
}

pub async fn backup_create(ctx: &Ctx, p: &Parsed) -> Result<Outcome, CliError> {
    ctx.require_install()?;
    if p.dry_run() {
        return Ok(Outcome::new(
            json!({ "dry_run": true, "request": { "method": "POST", "path": "/api/backups" } }),
            "would POST /api/backups",
        ));
    }
    let (token, _) = sign_token(ctx, None).await?;
    let v = into_result(
        Client::new(ctx)?
            .request("POST", "/api/backups", Some(&token), None)
            .await?,
    )?;
    let name = v
        .get("filename")
        .and_then(Value::as_str)
        .unwrap_or("?")
        .to_string();
    Ok(Outcome::new(v, format!("backup created: {name}")))
}
