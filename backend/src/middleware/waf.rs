use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use regex::Regex;
use std::net::SocketAddr;

use crate::application::AppState;

pub async fn waf_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let state = req.extensions().get::<AppState>().cloned();
    let Some(state) = state else {
        return Ok(next.run(req).await);
    };

    let client_ip = addr.ip().to_string();
    let uri = req.uri().path().to_string();
    let headers = req.headers().clone();

    // 1. Check IP rules
    if let Ok(rules) = state.waf_ip_repo.list_enabled().await {
        for rule in &rules {
            if ip_matches(&client_ip, &rule.ip) {
                if rule.action == "block" {
                    return Err((
                        StatusCode::FORBIDDEN,
                        format!("IP blocked: {}", rule.ip),
                    )
                        .into_response());
                }
                if rule.action == "allow" {
                    // Whitelisted - skip all further checks
                    return Ok(next.run(req).await);
                }
            }
        }
    }

    // 2. Check pattern-based rules against URL and headers
    if let Ok(rules) = state.waf_repo.list_enabled().await {
        for rule in &rules {
            let regex = match Regex::new(&rule.pattern) {
                Ok(r) => r,
                Err(_) => continue,
            };

            let matched = match rule.target.as_str() {
                "url" => regex.is_match(&uri),
                "header" => {
                    let header_str = format!("{:?}", headers);
                    regex.is_match(&header_str)
                }
                "cookie" => headers
                    .get("cookie")
                    .and_then(|v| v.to_str().ok())
                    .map(|c| regex.is_match(c))
                    .unwrap_or(false),
                "body" => false, // body inspection needs buffering, skip
                _ => false,
            };

            if matched {
                match rule.action.as_str() {
                    "block" => {
                        return Err((
                            StatusCode::FORBIDDEN,
                            format!("Blocked by WAF: {}", rule.name),
                        )
                            .into_response());
                    }
                    "log" => {
                        tracing::warn!(
                            "WAF '{}' matched: {} {} from {}",
                            rule.name,
                            req.method(),
                            uri,
                            client_ip
                        );
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(next.run(req).await)
}

fn ip_matches(client_ip: &str, rule_ip: &str) -> bool {
    if let Some((network, bits)) = rule_ip.split_once('/') {
        if let Ok(prefix_len) = bits.parse::<u8>() {
            if let (Ok(client), Ok(network_addr)) =
                (client_ip.parse::<std::net::IpAddr>(), network.parse::<std::net::IpAddr>())
            {
                return ip_in_cidr(&client, &network_addr, prefix_len);
            }
        }
    }
    client_ip == rule_ip
}

fn ip_in_cidr(ip: &std::net::IpAddr, network: &std::net::IpAddr, prefix_len: u8) -> bool {
    match (ip, network) {
        (std::net::IpAddr::V4(ip), std::net::IpAddr::V4(net)) => {
            let mask = u32::MAX.checked_shl(32 - prefix_len as u32).unwrap_or(0);
            ip.to_bits() & mask == net.to_bits() & mask
        }
        (std::net::IpAddr::V6(ip), std::net::IpAddr::V6(net)) => {
            let mask = u128::MAX.checked_shl(128 - prefix_len as u32).unwrap_or(0);
            ip.to_bits() & mask == net.to_bits() & mask
        }
        _ => false,
    }
}
