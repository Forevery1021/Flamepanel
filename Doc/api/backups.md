<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · backups

> 5 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/backups` | 备份列表 | `backup:read` |
| POST | `/api/backups` | 创建备份 | `backup:create` |
| GET | `/api/backups/{filename}` | 下载备份文件（流式） | `backup:read` |
| DELETE | `/api/backups/{filename}` | 删除备份 | `backup:delete` |
| POST | `/api/backups/{filename}/restore` | 从备份恢复数据库：先把当前库另存为 `pre-restore-*.db`，再原子替换库文件。 | `backup:create` |

### GET `/api/backups`

备份列表

- 认证：Bearer JWT
- 权限：`backup:read`
- operationId：`list_backups`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 备份列表 | BackupEntryDto[] |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "created_at": "string",
    "filename": "string",
    "size": 0
  }
]
```

</details>

```bash
fpctl api GET '/api/backups' --json
```

### POST `/api/backups`

创建备份

- 认证：Bearer JWT
- 权限：`backup:create`
- operationId：`create_backup`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 备份成功 | BackupEntryDto |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
{
  "created_at": "string",
  "filename": "string",
  "size": 0
}
```

</details>

```bash
fpctl api POST '/api/backups' --json
```

### GET `/api/backups/{filename}`

下载备份文件（流式）

- 认证：Bearer JWT
- 权限：`backup:read`
- operationId：`download_backup`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `filename` | path | string | 是 | 备份文件名 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 备份文件 | (`application/octet-stream`) |
| 401 | 未认证 |  |
| 404 | 备份不存在 |  |

```bash
fpctl api GET '/api/backups/1' --json
```

### DELETE `/api/backups/{filename}`

删除备份

- 认证：Bearer JWT
- 权限：`backup:delete`
- operationId：`delete_backup`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `filename` | path | string | 是 | 备份文件名 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 删除成功 |  |
| 404 | 备份不存在 |  |

```bash
fpctl api DELETE '/api/backups/1' --json
```

### POST `/api/backups/{filename}/restore`

从备份恢复数据库：先把当前库另存为 `pre-restore-*.db`，再原子替换库文件。

以请求体 `filename` 为准（路径段仅作路由）；运行中的连接仍指向旧文件，建议恢复后重启服务。

- 认证：Bearer JWT
- 权限：`backup:create`
- operationId：`restore_backup`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `filename` | path | string | 是 | 备份文件名 |

**请求体**（`application/json`，必填）：RestoreBackupRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `filename` | string | 是 |  |

```json
{
  "filename": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 恢复完成（建议随后重启服务） |  |
| 401 | 未认证 |  |
| 403 | 无权限 |  |
| 404 | 备份不存在 |  |

```bash
fpctl api POST '/api/backups/1/restore' -d '{"filename":"string"}' --json
```
