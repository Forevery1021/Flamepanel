use axum::{extract::State, Json, Router, routing::get};
use serde::Serialize;

use crate::application::AppState;
use crate::core::error::AppError;

#[derive(Debug, Serialize)]
struct GrafanaDashboard {
    title: String,
    uid: String,
    schema_version: u32,
    panels: Vec<GrafanaPanel>,
}

#[derive(Debug, Serialize)]
struct GrafanaPanel {
    id: u32,
    title: String,
    #[serde(rename = "type")]
    panel_type: String,
    datasource: GrafanaDatasource,
    targets: Vec<GrafanaTarget>,
    grid_pos: PanelPosition,
    field_config: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct GrafanaDatasource {
    #[serde(rename = "type")]
    ds_type: String,
    uid: String,
}

#[derive(Debug, Serialize)]
struct GrafanaTarget {
    expr: String,
    legend_format: String,
    #[serde(rename = "refId")]
    ref_id: String,
}

#[derive(Debug, Serialize)]
struct PanelPosition { x: u32, y: u32, w: u32, h: u32 }

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/grafana-dashboard", get(grafana_dashboard))
}

async fn grafana_dashboard(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let dashboard = GrafanaDashboard {
        title: "Flamepanel Server Monitor".into(),
        uid: "flamepanel-server".into(),
        schema_version: 39,
        panels: vec![
            stat_panel(1, "CPU Usage %", "flamepanel_cpu_usage_percent", 0, 0, 6, 4, "%"),
            stat_panel(2, "Memory Usage %", "flamepanel_memory_usage_percent", 6, 0, 6, 4, "%"),
            stat_panel(3, "Disk Usage %", "flamepanel_disk_usage_percent", 12, 0, 6, 4, "%"),
            stat_panel(4, "Load 1m", "flamepanel_load_one", 18, 0, 6, 4, ""),
            time_series_panel(5, "CPU Usage %", "flamepanel_cpu_usage_percent", 0, 4, 12, 8, "%"),
            time_series_panel(6, "Memory Usage %", "flamepanel_memory_usage_percent", 12, 4, 12, 8, "%"),
            time_series_panel(7, "Disk Usage %", "flamepanel_disk_usage_percent", 0, 12, 12, 8, "%"),
            multi_series_panel(8, "System Load Average",
                vec!["flamepanel_load_one", "flamepanel_load_five", "flamepanel_load_fifteen"],
                vec!["1m", "5m", "15m"],
                12, 12, 12, 8),
            time_series_panel(9, "Docker Containers Running", "flamepanel_docker_containers_running", 0, 20, 12, 6, ""),
            stat_panel(10, "Docker Total", "flamepanel_docker_containers_total", 12, 20, 6, 6, ""),
        ],
    };

    Ok(Json(serde_json::to_value(&dashboard).unwrap()))
}

fn stat_panel(id: u32, title: &str, expr: &str, x: u32, y: u32, w: u32, h: u32, unit: &str) -> GrafanaPanel {
    GrafanaPanel {
        id, title: title.into(),
        panel_type: "stat".into(),
        datasource: prometheus_ds(),
        targets: vec![GrafanaTarget {
            expr: expr.into(),
            legend_format: "__auto".into(),
            ref_id: "A".into(),
        }],
        grid_pos: PanelPosition { x, y, w, h },
        field_config: serde_json::json!({
            "defaults": {
                "unit": unit,
                "thresholds": { "steps": [] },
                "color": { "mode": "thresholds" },
            }
        }),
    }
}

fn time_series_panel(id: u32, title: &str, expr: &str, x: u32, y: u32, w: u32, h: u32, unit: &str) -> GrafanaPanel {
    GrafanaPanel {
        id, title: title.into(),
        panel_type: "timeseries".into(),
        datasource: prometheus_ds(),
        targets: vec![GrafanaTarget {
            expr: expr.into(),
            legend_format: "__auto".into(),
            ref_id: "A".into(),
        }],
        grid_pos: PanelPosition { x, y, w, h },
        field_config: serde_json::json!({
            "defaults": {
                "unit": unit,
                "custom": { "lineInterpolation": "smooth", "fillOpacity": 15 },
            }
        }),
    }
}

fn multi_series_panel(id: u32, title: &str, exprs: Vec<&str>, legends: Vec<&str>, x: u32, y: u32, w: u32, h: u32) -> GrafanaPanel {
    let targets: Vec<GrafanaTarget> = exprs.iter().enumerate().map(|(i, expr)| GrafanaTarget {
        expr: expr.to_string(),
        legend_format: legends.get(i).unwrap_or(&"").to_string(),
        ref_id: format!("{}", (b'A' + i as u8) as char),
    }).collect();

    GrafanaPanel {
        id, title: title.into(),
        panel_type: "timeseries".into(),
        datasource: prometheus_ds(),
        targets,
        grid_pos: PanelPosition { x, y, w, h },
        field_config: serde_json::json!({
            "defaults": {
                "custom": { "lineInterpolation": "smooth", "fillOpacity": 10 },
            }
        }),
    }
}

fn prometheus_ds() -> GrafanaDatasource {
    GrafanaDatasource {
        ds_type: "prometheus".into(),
        uid: "prometheus".into(),
    }
}
