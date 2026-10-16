<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · metrics

> 3 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/metrics/history` | `GET /api/metrics/history` — 监控历史降采样曲线（需 `node:read`） | `node:read` |
| GET | `/api/metrics/processes` | `GET /api/metrics/processes` — 按 CPU 占用排序的进程 TOP 5（免认证） F-17: processes 接口复用共享的 sysinfo::System 实例，避免每请求全量重建。 | `node:read` |
| GET | `/metrics` | `GET /metrics` — Prometheus 文本格式指标（面板自身系统指标） | 公开（无需登录） |

### GET `/api/metrics/history`

`GET /api/metrics/history` — 监控历史降采样曲线（需 `node:read`）

1 min 点由 3 s 采样在内存聚合后落库（保留 7 d），1 h 点由整点 rollup 得到（保留 90 d）；
响应带 `step_secs`，前端按桶宽绘制。刚启动或历史不足时点数少于窗口理论值。

- 认证：Bearer JWT
- 权限：`node:read`
- operationId：`history`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `range` | query | string? | 否 | 时间范围：`1h` / `6h` / `24h`（1 min 点）或 `7d` / `30d`（1 h 点）；缺省 `24h` |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 监控历史点列 | MetricsHistoryResponse |
| 400 | range 非法 |  |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
{
  "from": 0,
  "points": [
    {
      "bucket_start": 0,
      "cpu_avg": 0.0,
      "cpu_max": 0.0,
      "disk_avg": 0.0,
      "load_one_avg": 0.0,
      "memory_avg": 0.0,
      "memory_max": 0.0,
      "net_rx_avg_mbps": 0.0,
      "net_tx_avg_mbps": 0.0,
      "samples": 0
    }
  ],
  "range": "string",
  "step_secs": 0,
  "to": 0
}
```

</details>

```bash
fpctl api GET '/api/metrics/history' --json
```

### GET `/api/metrics/processes`

`GET /api/metrics/processes` — 按 CPU 占用排序的进程 TOP 5（免认证）
F-17: processes 接口复用共享的 sysinfo::System 实例，避免每请求全量重建。

- 认证：无
- 权限：`node:read`
- operationId：`processes`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 进程 TOP 列表 | ProcessEntry[] |

<details><summary>响应示例</summary>

```json
[
  {
    "cpu": 0.0,
    "memory_mb": 0,
    "name": "string",
    "pid": 0,
    "status": "string"
  }
]
```

</details>

### GET `/metrics`

`GET /metrics` — Prometheus 文本格式指标（面板自身系统指标）

Stage4.3：暴露 Prometheus 兼容的文本格式（`text/plain; version=0.0.4`），
供 Prometheus/Grafana 直接抓取。指标来自最近一次 `MetricsSnapshot`
（3s 周期采集）。公开可读（无敏感信息）。

- 认证：无
- 权限：公开（无需登录）
- operationId：`prometheus_metrics`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | Prometheus 文本格式指标 | (`text/plain`) |
