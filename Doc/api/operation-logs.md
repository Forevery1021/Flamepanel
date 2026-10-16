<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · operation-logs

> 3 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/operation-logs` | 操作审计日志列表 | `operation_log:read` |
| GET | `/api/operation-logs/export` | `GET /api/operation-logs/export?format=csv\|json` — 导出全部审计日志 | `operation_log:read` |
| DELETE | `/api/operation-logs/{id}` | 删除一条审计日志 | `operation_log:delete` |

### GET `/api/operation-logs`

操作审计日志列表

- 认证：Bearer JWT
- 权限：`operation_log:read`
- operationId：`list`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `page` | query | integer(int64)? | 否 | 页码（从 1 开始） |
| `page_size` | query | integer(int64)? | 否 | 每页条数（1~200） |
| `action` | query | string? | 否 | 按 action 前缀过滤（如 `action=LOGIN` 匹配 LOGIN_SUCCESS/LOGIN_FAILED） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 操作审计日志（分页，新在前） | PaginatedResponse<OperationLog>_OperationLog |
| 401 | 未认证 |  |
| 403 | 无权限 |  |

<details><summary>响应示例</summary>

```json
{
  "data": [
    {
      "action": "string",
      "created_at": "2026-01-01T00:00:00Z",
      "id": 0,
      "ip": "string",
      "target": "string",
      "username": "string"
    }
  ],
  "page": 0,
  "page_size": 0,
  "total": 0,
  "total_pages": 0
}
```

</details>

```bash
fpctl api GET '/api/operation-logs' --json
```

### GET `/api/operation-logs/export`

`GET /api/operation-logs/export?format=csv|json` — 导出全部审计日志

Stage4.4：支持 CSV / JSON 两种格式，便于归档与 SIEM 对接。
CSV 带 BOM（UTF-8）以兼容 Excel；JSON 为 UTF-8 数组。

- 认证：Bearer JWT
- 权限：`operation_log:read`
- operationId：`export`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `format` | query | string? | 否 |  |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 导出文件 |  |
| 401 | 未认证 |  |

```bash
fpctl api GET '/api/operation-logs/export' --json
```

### DELETE `/api/operation-logs/{id}`

删除一条审计日志

- 认证：Bearer JWT
- 权限：`operation_log:delete`
- operationId：`delete`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 删除成功 | string (`text/plain`) |
| 401 | 未认证 |  |
| 403 | 无权限 |  |
| 404 | 日志不存在 |  |

```bash
fpctl api DELETE '/api/operation-logs/1' --json
```
