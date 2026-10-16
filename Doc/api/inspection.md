<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · inspection

> 2 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/inspection` | `GET /api/inspection` — 系统体检报告（内核信息 + 风险项） | `toolbox:read` |
| POST | `/api/inspection/run` | `POST /api/inspection/run` — 立即重新体检（忽略缓存） | `toolbox:execute` |

### GET `/api/inspection`

`GET /api/inspection` — 系统体检报告（内核信息 + 风险项）

命中缓存时 `cache_age_secs > 0`；`findings[].code` 为稳定编码，前端据此展示三语言标题与建议，
`evidence` 为语言无关的原始取证。`heuristic = true` 的结论依赖内置生命周期表（`data_version`）。

- 认证：Bearer JWT
- 权限：`toolbox:read`
- operationId：`report`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 系统体检报告 | InspectionReport |
| 401 | 未认证 |  |
| 403 | 缺少 toolbox:read 权限 |  |

```bash
fpctl api GET '/api/inspection' --json
```

### POST `/api/inspection/run`

`POST /api/inspection/run` — 立即重新体检（忽略缓存）

采集有 15 s 总预算，单条外部命令 3 s 超时；超时或不可用的项以 `unknown` 呈现而不是让请求失败。

- 认证：Bearer JWT
- 权限：`toolbox:execute`
- operationId：`run`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 新生成的体检报告 | InspectionReport |
| 401 | 未认证 |  |
| 403 | 缺少 toolbox:execute 权限 |  |

```bash
fpctl api POST '/api/inspection/run' --json
```
