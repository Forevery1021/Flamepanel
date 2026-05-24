use axum::{
    extract::State,
    response::IntoResponse,
    routing::get,
    Router,
};

use crate::application::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/metrics", get(metrics))
}

async fn metrics(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let history = state.metrics_history.lock().await;
    let snapshot = history.get_all().last().cloned();

    let mut output = String::new();

    if let Some(s) = snapshot {
        output.push_str("# HELP flamepanel_cpu_usage_percent CPU usage percentage\n");
        output.push_str("# TYPE flamepanel_cpu_usage_percent gauge\n");
        output.push_str(&format!("flamepanel_cpu_usage_percent {:.2}\n", s.cpu_usage));

        output.push_str("# HELP flamepanel_cpu_cores Number of CPU cores\n");
        output.push_str("# TYPE flamepanel_cpu_cores gauge\n");
        output.push_str(&format!("flamepanel_cpu_cores {}\n", s.cpu_cores));

        output.push_str("# HELP flamepanel_memory_usage_percent Memory usage percentage\n");
        output.push_str("# TYPE flamepanel_memory_usage_percent gauge\n");
        output.push_str(&format!("flamepanel_memory_usage_percent {:.2}\n", s.memory_usage_percent));

        output.push_str("# HELP flamepanel_memory_total_mb Total memory in MB\n");
        output.push_str("# TYPE flamepanel_memory_total_mb gauge\n");
        output.push_str(&format!("flamepanel_memory_total_mb {}\n", s.memory_total_mb));

        output.push_str("# HELP flamepanel_memory_used_mb Used memory in MB\n");
        output.push_str("# TYPE flamepanel_memory_used_mb gauge\n");
        output.push_str(&format!("flamepanel_memory_used_mb {}\n", s.memory_used_mb));

        output.push_str("# HELP flamepanel_disk_usage_percent Disk usage percentage\n");
        output.push_str("# TYPE flamepanel_disk_usage_percent gauge\n");
        output.push_str(&format!("flamepanel_disk_usage_percent {:.2}\n", s.disk_usage_percent));

        output.push_str("# HELP flamepanel_disk_total_gb Total disk space in GB\n");
        output.push_str("# TYPE flamepanel_disk_total_gb gauge\n");
        output.push_str(&format!("flamepanel_disk_total_gb {:.2}\n", s.disk_total_gb));

        output.push_str("# HELP flamepanel_disk_used_gb Used disk space in GB\n");
        output.push_str("# TYPE flamepanel_disk_used_gb gauge\n");
        output.push_str(&format!("flamepanel_disk_used_gb {:.2}\n", s.disk_used_gb));

        output.push_str("# HELP flamepanel_load_one Load average 1min\n");
        output.push_str("# TYPE flamepanel_load_one gauge\n");
        output.push_str(&format!("flamepanel_load_one {:.2}\n", s.load_one));

        output.push_str("# HELP flamepanel_load_five Load average 5min\n");
        output.push_str("# TYPE flamepanel_load_five gauge\n");
        output.push_str(&format!("flamepanel_load_five {:.2}\n", s.load_five));

        output.push_str("# HELP flamepanel_load_fifteen Load average 15min\n");
        output.push_str("# TYPE flamepanel_load_fifteen gauge\n");
        output.push_str(&format!("flamepanel_load_fifteen {:.2}\n", s.load_fifteen));

        if let Some(gpu) = s.gpu_usage_percent {
            output.push_str("# HELP flamepanel_gpu_usage_percent GPU usage percentage\n");
            output.push_str("# TYPE flamepanel_gpu_usage_percent gauge\n");
            output.push_str(&format!("flamepanel_gpu_usage_percent {:.2}\n", gpu));
        }
        if let Some(gpu_mem) = s.gpu_memory_used_mb {
            output.push_str("# HELP flamepanel_gpu_memory_used_mb GPU memory used in MB\n");
            output.push_str("# TYPE flamepanel_gpu_memory_used_mb gauge\n");
            output.push_str(&format!("flamepanel_gpu_memory_used_mb {}\n", gpu_mem));
        }
        if let Some(gpu_temp) = s.gpu_temperature {
            output.push_str("# HELP flamepanel_gpu_temperature_celsius GPU temperature in Celsius\n");
            output.push_str("# TYPE flamepanel_gpu_temperature_celsius gauge\n");
            output.push_str(&format!("flamepanel_gpu_temperature_celsius {:.0}\n", gpu_temp));
        }
    }

    output.push_str("# HELP flamepanel_metrics_snapshots_total Total metrics snapshots collected\n");
    output.push_str("# TYPE flamepanel_metrics_snapshots_total counter\n");
    output.push_str(&format!("flamepanel_metrics_snapshots_total {}\n", history.get_all().len()));

    // Docker container counts
    use bollard::container::ListContainersOptions;
    if let Ok(containers) = state.docker.list_containers(Some(ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    })).await {
        let running = containers.iter().filter(|c| c.state.as_deref() == Some("running")).count();
        output.push_str("# HELP flamepanel_docker_containers_running Running Docker containers\n");
        output.push_str("# TYPE flamepanel_docker_containers_running gauge\n");
        output.push_str(&format!("flamepanel_docker_containers_running {}\n", running));
        output.push_str("# HELP flamepanel_docker_containers_total Total Docker containers\n");
        output.push_str("# TYPE flamepanel_docker_containers_total gauge\n");
        output.push_str(&format!("flamepanel_docker_containers_total {}\n", containers.len()));
    }

    (
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        output,
    )
}
