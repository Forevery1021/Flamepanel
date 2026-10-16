# 34 · 内置 MCP Server 接入指南（M-AI-1）

> 适用版本：1.1.0 起。对应路线图 Doc/30 §M-AI-1、执行计划 Doc/32 §2。
> 已合并：#162（后端：协议端点、凭证、12 个只读工具、调用审计与 REST 管理接口）、
> #164（前端「AI 接入中心」——系统 → AI 接入中心，`/ai-center`——与 `flamepanel mcp` stdio 模式）。

## 1. 概览

FlamePanel 内核内置一个 [Model Context Protocol](https://modelcontextprotocol.io) 服务端，
让 Claude Desktop / Claude Code / Cursor 等 AI 客户端**直接读取面板管理的服务器状态**，
例如「查 `web` 容器最近 100 行日志，判断是否 OOM」。

| 项 | 取值 |
|---|---|
| 端点 | `https://<面板地址>/api/mcp` |
| 传输 | Streamable HTTP（JSON-RPC 2.0，`POST` 返回 `application/json`；不提供 SSE 推送流） |
| 协议版本 | `2025-06-18`（兼容 `2025-03-26`、`2024-11-05`） |
| 鉴权 | `Authorization: Bearer fpk_…`（AI 接入凭证；**面板登录 JWT 不可用**） |
| 能力 | `tools`（12 个只读工具，见 §4） |

端点位于 `/api/` 下：沿用现有 nginx 反代、限流与「授权 IP / 绑定域名」访问控制，**部署无需任何改动**。

## 2. 安全模型

1. **凭证即 PAT**：`fpk_` + 64 位十六进制随机串（约 244 bit 熵）。明文只在签发时返回一次，
   库内只存 SHA-256 与展示前缀 `fpk_xxxxxxxx`。凭证可设 1–3650 天有效期或永不过期，可随时吊销。
2. **凭证不携带权限**：它绑定签发者用户，每次工具调用都按该用户**当前**角色做 RBAC。
   用户被删除、降权或进入强制改密状态会立即生效。任何人都只能签发不超过自己权限的凭证。
3. **权限与 REST 同源**：每个工具声明一个等价 REST 路由，所需权限从 REST 权限表推导
   （例如 `docker_logs` ≙ `GET /api/docker/containers/{id}/logs` → `docker:read`）。
   `tools/list` 只列出当前用户可用的工具。
4. **写操作必须人工批准（M-AI-2）**：凭证默认只读；签发时勾选「允许写操作」（`allow_write`）后写工具才可见。
   AI 调用写工具时面板只做**预览**（校验参数、确认目标存在、描述将发生的变更），生成审批单并把单号告诉模型，
   **不执行任何变更**。面板用户在「AI 接入中心 → 审批」批准后才以批准者身份执行，详见 §4.1。
5. **输出脱敏**：结构化输出中 `password` / `secret` / `token` / `api_key` / `credential` 等键的值，
   以及 `KEY=VALUE` 形式的环境变量值，一律替换为 `***`。已装应用不返回安装参数。
   `files_read` 拒绝读取私钥、`shadow`、`.env`、`.netrc`、`*.key`、`*.db` 等密钥类文件，
   请求路径与符号链接解析后的真实路径都会检查。
6. **防 DNS rebinding**：请求携带 `Origin` 时，其主机必须与 `Host`（或 `X-Forwarded-Host`）一致，否则返回 403。
7. **审计**：每次 `tools/call` 记录凭证、会话、工具、参数（已脱敏、截断到 2 KiB）、成败与耗时。
   保留最近 10 000 条。`files_read` 还会写一条操作日志 `mcp_file_read`。

权限点（新增，老库启动时自动补授）：

| 权限 | 默认角色 | 说明 |
|---|---|---|
| `mcp:read` | admin / operator / viewer | 查看凭证、会话、调用审计、工具目录 |
| `mcp:create` | admin / operator | 为自己签发凭证、吊销自己的凭证 |
| `mcp:update` | admin / operator | 批准 / 拒绝 AI 写操作（另需工具本身权限） |
| `mcp:delete` | admin | 吊销他人的凭证；批准高危（destructive）写操作 |

## 3. 接入步骤

### 3.1 签发凭证

推荐在面板「系统 → AI 接入中心」点击「签发凭证」：填写名称与有效期（留空为永不过期），
签发后弹窗**只显示一次**明文令牌，并按所选客户端（Cursor / Claude Code / Claude Desktop / curl 自检）
生成可直接粘贴的配置；点「我已保存」后明文即从页面丢弃。同页可吊销凭证、按凭证 / 工具筛选调用记录、
查看工具目录及当前角色是否可用。

也可以用 REST 签发（脚本化场景）：

```bash
curl -s -X POST https://panel.example.com/api/mcp/principals \
  -H "Authorization: Bearer <登录 access token>" \
  -H 'Content-Type: application/json' \
  -d '{"name":"cursor@laptop","expires_in_days":90}'
# → 201 {"principal":{…},"token":"fpk_…","endpoint":"/api/mcp"}   ← token 只显示这一次
```

### 3.2 Cursor

`~/.cursor/mcp.json`（或项目内 `.cursor/mcp.json`）：

```json
{
  "mcpServers": {
    "flamepanel": {
      "url": "https://panel.example.com/api/mcp",
      "headers": { "Authorization": "Bearer fpk_..." }
    }
  }
}
```

### 3.3 Claude Code

```bash
claude mcp add --transport http flamepanel https://panel.example.com/api/mcp \
  --header "Authorization: Bearer fpk_..."
```

### 3.4 Claude Desktop（`flamepanel mcp` stdio 桥）

Claude Desktop 的本地配置只能启动 stdio 进程。`flamepanel` 二进制自带桥接子命令
`flamepanel mcp`：从 stdin 逐行读取 JSON-RPC 消息，转发到面板的 `/api/mcp`，把响应逐行写回 stdout。

```json
{
  "mcpServers": {
    "flamepanel": {
      "command": "flamepanel",
      "args": ["mcp", "--url", "https://panel.example.com/api/mcp"],
      "env": { "FLAMEPANEL_MCP_TOKEN": "fpk_..." }
    }
  }
}
```

| 参数 / 环境变量 | 说明 |
| --- | --- |
| `--url <URL>` | 必填，面板 MCP 端点（`http(s)://…/api/mcp`） |
| `--insecure` / `-k` | 跳过 TLS 证书校验（仅限自签证书的内网面板） |
| `FLAMEPANEL_MCP_TOKEN` | 必填，`fpk_` 令牌。**不接受 `--token`**，避免令牌出现在 `ps` / 进程列表 |

行为约定：自动维护 `Mcp-Session-Id` 与 `MCP-Protocol-Version`（取自 `initialize` 结果）；通知得到 202 时不输出；
HTTP 错误或网络失败对每个带 `id` 的请求回 `-32000` 错误（通知静默），非 JSON 行回 `-32700`；
会话 404（面板重启或凭证被吊销）会清除本地会话并返回错误，客户端重新 `initialize` 即可；
stdin 关闭时发送 `DELETE` 结束会话。单次请求超时 45 s，响应上限 4 MiB。日志只写 stderr，不污染协议流。

本机未安装 flamepanel 时，仍可用 `mcp-remote` 桥接：

```json
{
  "mcpServers": {
    "flamepanel": {
      "command": "npx",
      "args": ["-y", "mcp-remote", "https://panel.example.com/api/mcp",
               "--header", "Authorization:${FP_TOKEN}"],
      "env": { "FP_TOKEN": "Bearer fpk_..." }
    }
  }
}
```

### 3.5 验收对话

> 「用 flamepanel 查一下 `web` 容器最近 100 行日志，判断它是不是被 OOM 杀掉的。」

预期：模型先调用 `docker_logs {container:"web", tail:100}`，再调用 `docker_inspect {container:"web"}`，
然后根据 `summary.oom_killed`、`exit_code`（137）和日志内容给出结论。两次调用都会出现在调用审计中。

## 4. 工具目录

| 工具 | 权限 | 参数 | 说明 |
|---|---|---|---|
| `system_status` | `node:read` | — | CPU / 内存 / 磁盘 / 负载 / 网络快照，面板版本 |
| `docker_list` | `docker:read` | — | 全部容器 |
| `docker_logs` | `docker:read` | `container`, `tail`(1–2000，默认 100) | 最近 N 行日志 |
| `docker_inspect` | `docker:read` | `container` | inspect（已脱敏）+ `summary`：状态、退出码、`oom_killed`、重启次数、内存上限 |
| `task_get` | `task:read` | `id` | 统一任务状态 / 进度 / 消息 |
| `logs_query` | `log:read` | `levels`, `q`, `target`, `limit`(≤500) | 面板内核日志（最新在前） |
| `files_read` | `file:read` | `path`, `max_bytes`(≤256 KiB，默认 64 KiB) | 读取文件管理根目录内文本文件 |
| `website_list` | `website:read` | — | 网站列表 |
| `website_config` | `website:read` | `id` | 网站完整配置 + 伪静态状态 |
| `database_list` | `database:read` | — | 数据库实例（不含口令） |
| `appstore_installed` | `app_store:read` | — | 已装应用（不含安装参数） |
| `firewall_rules` | `firewall:read` | — | 防火墙规则 |
| `approval_get` | `mcp:read` | `id` | 查询本凭证发起的审批单状态、预览与执行结果 / 拒绝理由 |
| `backup_job_list` | `backup:read` | — | 备份任务：范围、计划、目标账号、保留份数、最近执行状态 / 信息 |
| `scheduled_task_list` | `scheduled_task:read` | — | 定时任务（≤ 200）：计划、最近状态、输出摘要；命令中的密码类参数脱敏 |

**写工具（M-AI-2，需 `allow_write` 凭证 + 人工审批）**：

| 工具 | 风险 | 权限 | 参数 | 说明 |
|---|---|---|---|---|
| `docker_container_start` | write | `docker:start` | `container` | 启动容器 |
| `docker_container_stop` | write | `docker:stop` | `container`, `timeout`(1–300，默认 10) | 停止容器 |
| `docker_container_restart` | write | `docker:start` | `container`, `timeout` | 重启容器 |
| `website_deploy` | write | `website:update` | `id` | 按面板配置重新渲染并下发站点，校验通过后 reload |
| `docker_container_remove` | destructive | `docker:delete` | `container`, `force` | 删除容器（镜像与数据卷保留）；运行中需 `force=true` |
| `app_native_restart` | write | `app_store:update` | `id` | 重启原生模式安装的应用（容器应用预览时提示改用 `docker_container_restart`） |
| `backup_create` | write | `backup:create` | — | 立即为面板数据库创建本地快照备份 |
| `backup_job_run` | write | `backup:update` | `id` | 立即执行一次备份任务；批准后**后台执行**（不受 120 s 执行时限约束），已在执行中 409，结果见 `backup_job_list` |
| `scheduled_task_run` | destructive | `scheduled_task:execute` | `id` | 立即执行定时任务命令（≤ 60 s），同步返回状态与输出；同一任务运行中拒绝。命令由管理员预先配置，AI 不能修改；预览展示脱敏后的命令 |

### 4.1 审批流程

1. AI 调用写工具 → 面板预览（容器 inspect / 网站读取）。预览失败（参数错误、目标不存在、运行中容器未带 `force`）
   直接返回工具级错误，**不生成审批单**。
2. 预览成功 → 生成审批单（`pending`，30 分钟过期；每凭证最多 20 个待审批），工具返回
   「已提交审批单 #N，尚未执行」与预览文本。工具注解按风险输出 `readOnlyHint` / `destructiveHint`。
3. 面板「审批」页签（带待审批角标）→ **批准**：校验批准者 `mcp:update` + 工具本身权限（高危另需 `mcp:delete`）、
   凭证仍有效且可写、签发者当前角色仍具备该权限 → `pending → executing`（比较并设置，并发批准只执行一次）→
   执行（120 s 超时）→ `succeeded` / `failed`，结果（≤ 8 KiB）写回审批单。**拒绝**可附理由。
4. AI 用 `approval_get {"id": N}` 查询结果。执行同时记入工具调用审计与操作日志（`mcp_approve` / `mcp_reject`）。

权限以 `GET /api/mcp/tools` 返回的 `permission` 字段为准。该字段由 REST 权限表实时推导，本表是 1.1.0 的推导结果。

工具执行超时 30 s，单次输出上限 100 KiB（超出截断并提示）。参数错误、权限不足、目标不存在等情况
返回**工具级错误**（`isError: true`，文本说明原因），不返回协议错误，模型可以据此自行调整参数。

### 4.2 Prompts 运维技能库

`prompts/list` / `prompts/get` 提供 7 个 Runbook 提示词，客户端通常显示为斜杠命令；面板助手的「运维技能」按钮使用同一份内容
（REST `GET /api/mcp/prompts`、`GET /api/mcp/prompts/{name}?<参数>`，`mcp:read`）。

| 名称 | 参数 | 用途 |
|---|---|---|
| `container_diagnose` | `container`* | 容器退出 / 反复重启 / OOM 排查 |
| `website_unreachable` | `website`*（域名或 ID） | 网站 502 / 无法访问逐层排查 |
| `host_health_check` | — | 主机健康巡检（只读） |
| `error_log_triage` | `keyword`, `target` | 错误日志聚类归因 |
| `security_review` | — | 安全基线检查（暴露端口、特权容器、登录失败） |
| `pre_change_checkpoint` | `change`* | 变更前记录现状、备份、写回滚方案 |
| `backup_task_review` | — | 备份与定时任务执行复盘 |

技能只是提示词：每段末尾附统一约束（先取证、变更只能经审批工具提出、不索要密钥），执行时调用的工具仍逐个鉴权与审批。
参数值去控制字符并截断到 200 字符；缺必填参数 / 传未声明参数 → `-32602`（REST 为 400）。
单测保证模板引用的工具名都存在于工具目录。

## 5. 协议细节

| 请求 | 响应 |
|---|---|
| `POST` `initialize` | 200，响应头 `Mcp-Session-Id: <uuid>` |
| `POST` 仅通知（如 `notifications/initialized`） | 202，无响应体 |
| `POST` `ping` / `tools/list` / `tools/call` / `prompts/list` / `prompts/get` | 200 `application/json`；支持批量数组（≤ 32 条） |
| `POST` 携带无效 / 已终止 / 他人的 `Mcp-Session-Id` | 404（客户端应重新 `initialize`） |
| `POST` 不带 `Mcp-Session-Id` | 按无状态模式处理（审计中会话为空） |
| `DELETE` + `Mcp-Session-Id` | 204，终止会话 |
| `GET` | 405（不提供服务端推送流） |
| 凭证缺失 / 无效 / 吊销 / 过期 | 401 |

错误码：`-32700` 解析失败、`-32600` 非法请求、`-32601` 方法不存在、`-32602` 参数错误（含未知工具）、`-32603` 内部错误。

手工调试：

```bash
T=fpk_...; U=https://panel.example.com/api/mcp
SID=$(curl -si $U -H "Authorization: Bearer $T" -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"curl","version":"1"}}}' \
  | awk -F': ' 'tolower($1)=="mcp-session-id"{print $2}' | tr -d '\r')
curl -s $U -H "Authorization: Bearer $T" -H "Mcp-Session-Id: $SID" -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"docker_list","arguments":{}}}'
```

## 6. 管理接口

| 端点 | 权限 | 说明 |
|---|---|---|
| `GET /api/mcp/tools` | `mcp:read` | 工具目录（`permission`、当前用户 `allowed`、`input_schema`） |
| `GET /api/mcp/prompts` | `mcp:read` | 运维技能库（与 `prompts/list` 一致） |
| `GET /api/mcp/prompts/{name}?<参数>` | `mcp:read` | 渲染技能 → `{name, title, text}`；缺必填参数 400、未知技能 404 |
| `GET /api/mcp/principals` | `mcp:read` | 凭证列表（`owner`、`active`、`last_used_at`；不含令牌） |
| `POST /api/mcp/principals` | `mcp:create` | `{name, expires_in_days?}` → 201，含一次性 `token` |
| `POST /api/mcp/principals/{id}/revoke` | `mcp:create`（他人凭证另需 `mcp:delete`） | 吊销（幂等），同时关闭其全部会话 |
| `GET /api/mcp/sessions?principal_id&limit` | `mcp:read` | 会话（最近活跃在前） |
| `GET /api/mcp/calls?principal_id&session_id&tool&page&page_size` | `mcp:read` | 调用审计分页 |
| `GET /api/mcp/approvals?status&principal_id&page&page_size` | `mcp:read` | 审批单分页 |
| `GET /api/mcp/approvals/{id}` | `mcp:read` | 审批单详情 |
| `POST /api/mcp/approvals/{id}/approve` | `mcp:update`（+ 工具权限；高危 + `mcp:delete`） | 批准并执行 |
| `POST /api/mcp/approvals/{id}/reject` | `mcp:update` | 拒绝（`{reason?}`） |

数据表：`agent_principals`、`agent_sessions`、`tool_calls`、`tool_approvals`（启动时幂等建表）。

## 7. 后续

- **M-AI-2（1.2.0）已交付**：确认分级 + 审批工作流、首批 5 个写工具（#166）。
- **M-AI-2 Batch 2 + Prompts（1.3.0 开发中）已交付**：4 个写工具（原生应用重启、面板备份、备份任务、定时任务）+ 2 个配套只读工具；7 个 Runbook 技能。
- **剩余**：按凭证收窄权限范围、工具目录与 `fpctl commands --json`（`cli/catalog.rs`）统一。
