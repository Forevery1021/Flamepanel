<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · files

> 18 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/files` | `GET /api/files` — 列出目录 | `file:read` |
| POST | `/api/files/chmod` | `POST /api/files/chmod` — 修改权限 | `file:write` |
| POST | `/api/files/compress` | `POST /api/files/compress` — 压缩 | `file:write` |
| POST | `/api/files/copy` | `POST /api/files/copy` — 批量复制到目录 | `file:write` |
| POST | `/api/files/create-dir` | `POST /api/files/create-dir` — 新建目录 | `file:write` |
| POST | `/api/files/create-file` | `POST /api/files/create-file` — 新建空文件 | `file:write` |
| DELETE | `/api/files/delete` | `DELETE /api/files/delete` — 删除（默认移入回收站） | `file:write` |
| GET | `/api/files/download` | `GET /api/files/download` — 下载文件（流式） | `file:upload` |
| POST | `/api/files/extract` | `POST /api/files/extract` — 解压 | `file:write` |
| POST | `/api/files/move` | `POST /api/files/move` — 批量移动到目录 | `file:write` |
| GET | `/api/files/read` | `GET /api/files/read` — 读取文本文件（≤ 10 MiB，UTF-8） | `file:read` |
| GET | `/api/files/recycle` | `GET /api/files/recycle` — 回收站列表 | `file:read` |
| DELETE | `/api/files/recycle` | `DELETE /api/files/recycle` — 清空回收站 | `file:write` |
| DELETE | `/api/files/recycle/{id}` | `DELETE /api/files/recycle/{id}` — 彻底删除单个条目 | `file:write` |
| POST | `/api/files/recycle/{id}/restore` | `POST /api/files/recycle/{id}/restore` — 还原到原位置 | `file:write` |
| POST | `/api/files/rename` | `POST /api/files/rename` — 重命名 / 移动 | `file:write` |
| POST | `/api/files/upload` | `POST /api/files/upload` — 上传文件（请求体为文件原始字节） | `file:upload` |
| POST | `/api/files/write` | `POST /api/files/write` — 保存文件（原子替换，保留权限与属主） | `file:write` |

### GET `/api/files`

`GET /api/files` — 列出目录

- 认证：Bearer JWT
- 权限：`file:read`
- operationId：`list`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `path` | query | string? | 否 | 目录（根内相对路径，缺省 `/`） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 目录条目（目录在前，按名称排序；回收站目录不出现） | FileInfo[] |
| 400 | 不是目录 |  |
| 401 | 未认证 |  |
| 403 | 缺少 file:read 权限或路径越界 |  |
| 404 | 路径不存在 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "is_dir": false,
    "is_symlink": false,
    "mime_type": "string",
    "modified_at": "string",
    "name": "string",
    "path": "string",
    "permissions": "string",
    "size": 0
  }
]
```

</details>

```bash
fpctl api GET '/api/files' --json
```

### POST `/api/files/chmod`

`POST /api/files/chmod` — 修改权限

- 认证：Bearer JWT
- 权限：`file:write`
- operationId：`chmod`

**请求体**（`application/json`，必填）：ChmodRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `mode` | string | 是 | 八进制权限，如 `644` / `0755` |
| `path` | string | 是 |  |

```json
{
  "mode": "string",
  "path": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已修改 | string (`text/plain`) |
| 400 | 权限值非法 |  |
| 401 | 未认证 |  |
| 403 | 缺少 file:write 权限或路径越界 |  |
| 404 | 路径不存在 |  |

```bash
fpctl api POST '/api/files/chmod' -d '{"mode":"string","path":"string"}' --json
```

### POST `/api/files/compress`

`POST /api/files/compress` — 压缩

- 认证：Bearer JWT
- 权限：`file:write`
- operationId：`compress`

**请求体**（`application/json`，必填）：CompressRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `dest` | string | 是 | 压缩包路径，扩展名决定格式：`.tar.gz` / `.tgz` / `.zip` |
| `overwrite` | boolean | 否 |  |
| `paths` | string[] | 是 | 同一目录下的若干条目 |

```json
{
  "dest": "string",
  "overwrite": false,
  "paths": [
    "string"
  ]
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已生成压缩包 | CompressResult |
| 400 | 未选择条目 / 条目不在同一目录 / 扩展名不支持 / 超出 zip 限制 |  |
| 401 | 未认证 |  |
| 403 | 缺少 file:write 权限或路径越界 |  |
| 404 | 条目不存在 |  |
| 409 | 压缩包已存在（未指定 overwrite） |  |

<details><summary>响应示例</summary>

```json
{
  "entries": 0,
  "path": "string",
  "size": 0,
  "skipped": 0
}
```

</details>

```bash
fpctl api POST '/api/files/compress' -d '{"dest":"string","overwrite":false,"paths":["string"]}' --json
```

### POST `/api/files/copy`

`POST /api/files/copy` — 批量复制到目录

- 认证：Bearer JWT
- 权限：`file:write`
- operationId：`copy_entries`

**请求体**（`application/json`，必填）：TransferRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `dest_dir` | string | 是 | 目标目录（必须已存在） |
| `overwrite` | boolean | 否 | 同名条目存在时覆盖（旧条目移入回收站）；缺省 false → 该项失败（`error_kind = conflict`） |
| `sources` | string[] | 是 | 源条目（根内路径，可来自不同目录；单次 ≤ 500 项） |

```json
{
  "dest_dir": "string",
  "overwrite": false,
  "sources": [
    "string"
  ]
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 逐项结果（部分失败仍为 200，见 items[].ok / error_kind） | TransferResult |
| 400 | 未选择条目 / 超过 500 项 / 目标不是目录 |  |
| 401 | 未认证 |  |
| 403 | 缺少 file:write 权限或目标目录越界 |  |
| 404 | 目标目录不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "failed": 0,
  "items": [
    {
      "error": "string",
      "error_kind": "string",
      "ok": false,
      "source": "string",
      "target": "string"
    }
  ],
  "succeeded": 0
}
```

</details>

```bash
fpctl api POST '/api/files/copy' -d '{"dest_dir":"string","overwrite":false,"sources":["string"]}' --json
```

### POST `/api/files/create-dir`

`POST /api/files/create-dir` — 新建目录

- 认证：Bearer JWT
- 权限：`file:write`
- operationId：`create_dir`

**请求体**（`application/json`，必填）：CreateRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `path` | string | 是 |  |

```json
{
  "path": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已创建 | string (`text/plain`) |
| 400 | 路径非法或父目录不存在 |  |
| 401 | 未认证 |  |
| 403 | 缺少 file:write 权限或路径越界 |  |
| 409 | 同名条目已存在 |  |

```bash
fpctl api POST '/api/files/create-dir' -d '{"path":"string"}' --json
```

### POST `/api/files/create-file`

`POST /api/files/create-file` — 新建空文件

- 认证：Bearer JWT
- 权限：`file:write`
- operationId：`create_file`

**请求体**（`application/json`，必填）：CreateRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `path` | string | 是 |  |

```json
{
  "path": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已创建 | string (`text/plain`) |
| 400 | 路径非法或父目录不存在 |  |
| 401 | 未认证 |  |
| 403 | 缺少 file:write 权限或路径越界 |  |
| 409 | 同名条目已存在 |  |

```bash
fpctl api POST '/api/files/create-file' -d '{"path":"string"}' --json
```

### DELETE `/api/files/delete`

`DELETE /api/files/delete` — 删除（默认移入回收站）

- 认证：Bearer JWT
- 权限：`file:write`
- operationId：`delete`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `path` | query | string | 是 |  |
| `recursive` | query | boolean? | 否 | 非空目录必须为 true |
| `permanent` | query | boolean? | 否 | true = 直接删除，不进回收站 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已删除（或已移入回收站） | string (`text/plain`) |
| 400 | 目录非空且未确认递归 / 位于其他文件系统无法移入回收站 |  |
| 401 | 未认证 |  |
| 403 | 缺少 file:write 权限、路径越界或试图删除根目录 |  |
| 404 | 路径不存在 |  |

```bash
fpctl api DELETE '/api/files/delete' --json
```

### GET `/api/files/download`

`GET /api/files/download` — 下载文件（流式）

- 认证：Bearer JWT
- 权限：`file:upload`
- operationId：`download`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `path` | query | string | 是 | 文件路径（根内相对路径） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 文件内容 | (`application/octet-stream`) |
| 400 | 不是文件 |  |
| 401 | 未认证 |  |
| 403 | 缺少 file:upload 权限或路径越界 |  |
| 404 | 路径不存在 |  |

```bash
fpctl api GET '/api/files/download' --json
```

### POST `/api/files/extract`

`POST /api/files/extract` — 解压

- 认证：Bearer JWT
- 权限：`file:write`
- operationId：`extract`

**请求体**（`application/json`，必填）：ExtractRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `dest` | string? | 否 | 解压到的目录（缺省为压缩包所在目录） |
| `overwrite` | boolean | 否 | 同名顶层条目存在时替换（旧条目移入回收站）；缺省 false → 409 且不做改动 |
| `path` | string | 是 | 压缩包路径（zip / tar / tar.gz / tgz / tar.xz / txz / gz） |

```json
{
  "dest": "string",
  "overwrite": false,
  "path": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已解压 | ExtractResult |
| 400 | 格式不支持 / 压缩包损坏 / 含路径穿越、包外符号链接、硬链接 / 超出体积与条目上限 |  |
| 401 | 未认证 |  |
| 403 | 缺少 file:write 权限或路径越界 |  |
| 404 | 压缩包不存在 |  |
| 409 | 目标目录已有同名顶层条目（未指定 overwrite，未做任何改动） |  |

<details><summary>响应示例</summary>

```json
{
  "bytes": 0,
  "entries": 0,
  "replaced": [
    "string"
  ],
  "skipped": 0,
  "top_level": [
    "string"
  ]
}
```

</details>

```bash
fpctl api POST '/api/files/extract' -d '{"dest":"string","overwrite":false,"path":"string"}' --json
```

### POST `/api/files/move`

`POST /api/files/move` — 批量移动到目录

- 认证：Bearer JWT
- 权限：`file:write`
- operationId：`move_entries`

**请求体**（`application/json`，必填）：TransferRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `dest_dir` | string | 是 | 目标目录（必须已存在） |
| `overwrite` | boolean | 否 | 同名条目存在时覆盖（旧条目移入回收站）；缺省 false → 该项失败（`error_kind = conflict`） |
| `sources` | string[] | 是 | 源条目（根内路径，可来自不同目录；单次 ≤ 500 项） |

```json
{
  "dest_dir": "string",
  "overwrite": false,
  "sources": [
    "string"
  ]
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 逐项结果（部分失败仍为 200，见 items[].ok / error_kind） | TransferResult |
| 400 | 未选择条目 / 超过 500 项 / 目标不是目录 |  |
| 401 | 未认证 |  |
| 403 | 缺少 file:write 权限或目标目录越界 |  |
| 404 | 目标目录不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "failed": 0,
  "items": [
    {
      "error": "string",
      "error_kind": "string",
      "ok": false,
      "source": "string",
      "target": "string"
    }
  ],
  "succeeded": 0
}
```

</details>

```bash
fpctl api POST '/api/files/move' -d '{"dest_dir":"string","overwrite":false,"sources":["string"]}' --json
```

### GET `/api/files/read`

`GET /api/files/read` — 读取文本文件（≤ 10 MiB，UTF-8）

- 认证：Bearer JWT
- 权限：`file:read`
- operationId：`read`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `path` | query | string | 是 | 文件路径（根内相对路径） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 文件内容 | string (`text/plain`) |
| 400 | 不是文件 / 超过 10 MiB / 不是 UTF-8 文本 |  |
| 401 | 未认证 |  |
| 403 | 缺少 file:read 权限或路径越界 |  |
| 404 | 路径不存在 |  |

```bash
fpctl api GET '/api/files/read' --json
```

### GET `/api/files/recycle`

`GET /api/files/recycle` — 回收站列表

- 认证：Bearer JWT
- 权限：`file:read`
- operationId：`recycle_list`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 回收站条目（新删除的在前；过期条目自动清理） | RecycleItem[] |
| 401 | 未认证 |  |
| 403 | 缺少 file:read 权限 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "deleted_at": "2026-01-01T00:00:00Z",
    "deleted_by": "string",
    "expires_at": "2026-01-01T00:00:00Z",
    "id": "string",
    "is_dir": false,
    "name": "string",
    "original_path": "string",
    "size": 0,
    "size_truncated": false
  }
]
```

</details>

```bash
fpctl api GET '/api/files/recycle' --json
```

### DELETE `/api/files/recycle`

`DELETE /api/files/recycle` — 清空回收站

- 认证：Bearer JWT
- 权限：`file:write`
- operationId：`recycle_empty`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已清空 | PurgeResult |
| 401 | 未认证 |  |
| 403 | 缺少 file:write 权限 |  |

<details><summary>响应示例</summary>

```json
{
  "purged": 0
}
```

</details>

```bash
fpctl api DELETE '/api/files/recycle' --json
```

### DELETE `/api/files/recycle/{id}`

`DELETE /api/files/recycle/{id}` — 彻底删除单个条目

- 认证：Bearer JWT
- 权限：`file:write`
- operationId：`recycle_purge`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 回收站条目 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已彻底删除 | string (`text/plain`) |
| 400 | ID 非法 |  |
| 401 | 未认证 |  |
| 403 | 缺少 file:write 权限 |  |
| 404 | 条目不存在 |  |

```bash
fpctl api DELETE '/api/files/recycle/1' --json
```

### POST `/api/files/recycle/{id}/restore`

`POST /api/files/recycle/{id}/restore` — 还原到原位置

- 认证：Bearer JWT
- 权限：`file:write`
- operationId：`recycle_restore`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 回收站条目 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已还原 | RecycleItem |
| 400 | ID 非法 |  |
| 401 | 未认证 |  |
| 403 | 缺少 file:write 权限 |  |
| 404 | 条目不存在 |  |
| 409 | 原位置已存在同名条目 |  |

<details><summary>响应示例</summary>

```json
{
  "deleted_at": "2026-01-01T00:00:00Z",
  "deleted_by": "string",
  "expires_at": "2026-01-01T00:00:00Z",
  "id": "string",
  "is_dir": false,
  "name": "string",
  "original_path": "string",
  "size": 0,
  "size_truncated": false
}
```

</details>

```bash
fpctl api POST '/api/files/recycle/1/restore' --json
```

### POST `/api/files/rename`

`POST /api/files/rename` — 重命名 / 移动

- 认证：Bearer JWT
- 权限：`file:write`
- operationId：`rename`

**请求体**（`application/json`，必填）：RenameRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `new_path` | string | 是 |  |
| `old_path` | string | 是 |  |

```json
{
  "new_path": "string",
  "old_path": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已重命名 | string (`text/plain`) |
| 400 | 路径非法或把目录移入自身 |  |
| 401 | 未认证 |  |
| 403 | 缺少 file:write 权限或路径越界 |  |
| 404 | 源路径不存在 |  |
| 409 | 目标已存在 |  |

```bash
fpctl api POST '/api/files/rename' -d '{"new_path":"string","old_path":"string"}' --json
```

### POST `/api/files/upload`

`POST /api/files/upload` — 上传文件（请求体为文件原始字节）

- 认证：Bearer JWT
- 权限：`file:upload`
- operationId：`upload`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `path` | query | string | 是 | 目标目录 |
| `name` | query | string | 是 | 文件名（不含路径） |
| `overwrite` | query | boolean? | 否 | 同名文件存在时是否覆盖（缺省 false → 409） |

**请求体**（`application/octet-stream`，必填）：integer(int32)[]

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已上传 | string (`text/plain`) |
| 400 | 文件名非法 / 目标不是目录 / 超过 2 GiB |  |
| 401 | 未认证 |  |
| 403 | 缺少 file:upload 权限或路径越界 |  |
| 409 | 同名文件已存在（未指定 overwrite） |  |

```bash
fpctl api POST '/api/files/upload' --json
```

### POST `/api/files/write`

`POST /api/files/write` — 保存文件（原子替换，保留权限与属主）

- 认证：Bearer JWT
- 权限：`file:write`
- operationId：`write`

**请求体**（`application/json`，必填）：WriteRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `content` | string | 是 |  |
| `expected_modified` | string? | 否 | 打开文件时看到的 `modified_at`；与当前不一致返回 409（缺省 = 强制覆盖） |
| `path` | string | 是 |  |

```json
{
  "content": "string",
  "expected_modified": "string",
  "path": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已保存，返回新的修改时间 | WriteResult |
| 400 | 不是文件 |  |
| 401 | 未认证 |  |
| 403 | 缺少 file:write 权限或路径越界 |  |
| 404 | 路径不存在 |  |
| 409 | 文件在打开后已被修改 |  |

<details><summary>响应示例</summary>

```json
{
  "modified_at": "string",
  "size": 0
}
```

</details>

```bash
fpctl api POST '/api/files/write' -d '{"content":"string","expected_modified":"string","path":"string"}' --json
```
