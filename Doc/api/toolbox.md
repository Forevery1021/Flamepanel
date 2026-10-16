<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · toolbox

> 3 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/toolbox/host` | `GET /api/toolbox/host` — 主机详情 | `toolbox:read` |
| GET | `/api/toolbox/processes` | `GET /api/toolbox/processes` — 进程列表 | `toolbox:read` |
| POST | `/api/toolbox/processes/{pid}/kill` | `POST /api/toolbox/processes/{pid}/kill` — 向进程发送信号 | `toolbox:execute` |

### GET `/api/toolbox/host`

`GET /api/toolbox/host` — 主机详情

- 认证：Bearer JWT
- 权限：`toolbox:read`
- operationId：`host_details`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 主机详情：系统 / CPU / 内存 / 时区 / DNS / hosts / 网卡 / 磁盘 / 面板自身 | HostDetails |
| 401 | 未认证 |  |
| 403 | 缺少 toolbox:read 权限 |  |

```bash
fpctl api GET '/api/toolbox/host' --json
```

### GET `/api/toolbox/processes`

`GET /api/toolbox/processes` — 进程列表

- 认证：Bearer JWT
- 权限：`toolbox:read`
- operationId：`list_processes`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `q` | query | string? | 否 | 关键字：pid 精确匹配，或进程名 / 用户 / 命令行包含（不区分大小写） |
| `user` | query | string? | 否 | 仅显示该用户的进程 |
| `sort` | query | string? | 否 | 排序字段：`cpu`（缺省）/ `memory` / `pid` / `name` / `started` |
| `order` | query | string? | 否 | 排序方向：`desc`（缺省）/ `asc` |
| `limit` | query | integer? | 否 | 返回条数：缺省 100，范围 1–500 |
| `include_kernel` | query | boolean? | 否 | 是否包含内核线程（缺省 false） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 进程列表 | ProcessListResponse |
| 400 | sort / order 非法 |  |
| 401 | 未认证 |  |
| 403 | 缺少 toolbox:read 权限 |  |

<details><summary>响应示例</summary>

```json
{
  "items": [
    {
      "command": null,
      "cpu": null,
      "kernel_thread": null,
      "memory_bytes": null,
      "name": null,
      "pid": null,
      "ppid": null,
      "started_at": null,
      "status": null,
      "uid": null,
      "user": null,
      "protected": null
    }
  ],
  "matched": 0,
  "panel_pid": 0,
  "sampled_at": 0,
  "total": 0,
  "total_memory_bytes": 0
}
```

</details>

```bash
fpctl api GET '/api/toolbox/processes' --json
```

### POST `/api/toolbox/processes/{pid}/kill`

`POST /api/toolbox/processes/{pid}/kill` — 向进程发送信号

- 认证：Bearer JWT
- 权限：`toolbox:execute`
- operationId：`kill_process`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `pid` | path | integer(int32) | 是 | 目标进程 pid |

**请求体**（`application/json`，必填）：KillProcessRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `signal` | ProcessSignal | 否 | 信号：`TERM`（缺省）/ `KILL` / `HUP` / `INT` |
| `started_at` | integer(int64)? | 否 | 列表中看到的启动时间（Unix 秒）；与当前进程不一致时返回 409，防止误杀复用了 pid 的新进程 |

```json
{
  "signal": "TERM",
  "started_at": 0
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 信号已送达 | KillProcessResponse |
| 400 | 受保护进程（init / 内核线程 / 面板自身及祖先）或请求体非法 |  |
| 401 | 未认证 |  |
| 403 | 缺少 toolbox:execute 权限；或面板系统权限不足（message 含 insufficient privilege） |  |
| 404 | 进程不存在 |  |
| 409 | started_at 不一致（pid 已被复用） |  |

<details><summary>响应示例</summary>

```json
{
  "elevated": false,
  "name": "string",
  "pid": 0,
  "signal": "TERM"
}
```

</details>

```bash
fpctl api POST '/api/toolbox/processes/1/kill' -d '{"signal":"TERM","started_at":0}' --json
```
