/**
 * MCP / AI 接入中心 API（M-AI-1 · Doc/34）。
 * 类型来自 OpenAPI 生成（src/api/generated），禁止手写重复定义。
 * 协议端点 `/api/mcp` 由 AI 客户端以 `fpk_` 令牌直连，本模块只封装管理接口。
 */
import api from './client'
import type {
  AgentSession,
  CreatePrincipalRequest,
  CreatePrincipalResponse,
  McpPromptInfo,
  McpPromptRendered,
  PrincipalView,
  ToolApproval,
  ToolCallRecord,
  ToolInfo,
} from '@/api/generated'

export type {
  AgentSession,
  CreatePrincipalRequest,
  CreatePrincipalResponse,
  McpPromptInfo,
  McpPromptRendered,
  PrincipalView,
  ToolApproval,
  ToolCallRecord,
  ToolInfo,
}

export interface ApprovalPage {
  data: ToolApproval[]
  page: number
  page_size: number
  total: number
  total_pages: number
}

export interface ApprovalQuery {
  status?: string | null
  principal_id?: number | null
  page?: number
  page_size?: number
}

export interface ToolCallPage {
  data: ToolCallRecord[]
  page: number
  page_size: number
  total: number
  total_pages: number
}

export interface ToolCallQuery {
  principal_id?: number | null
  session_id?: string | null
  tool?: string | null
  page?: number
  page_size?: number
}

/** 工具目录（含当前用户是否可用） */
export function listMcpTools() {
  return api.get<ToolInfo[]>('/mcp/tools')
}

/** 运维技能库（Runbook 提示词，与 MCP prompts/list 一致） */
export function listMcpPrompts() {
  return api.get<McpPromptInfo[]>('/mcp/prompts')
}

/** 渲染运维技能：参数以查询串传递，空值不传 */
export function renderMcpPrompt(name: string, args: Record<string, string> = {}) {
  const params = Object.fromEntries(Object.entries(args).filter(([, v]) => v.trim() !== ''))
  return api.get<McpPromptRendered>(`/mcp/prompts/${encodeURIComponent(name)}`, { params })
}

/** 凭证列表 */
export function listPrincipals() {
  return api.get<PrincipalView[]>('/mcp/principals')
}

/** 签发凭证：明文令牌仅在响应中出现这一次 */
export function createPrincipal(input: CreatePrincipalRequest) {
  return api.post<CreatePrincipalResponse>('/mcp/principals', input)
}

/** 吊销凭证（同时关闭其会话） */
export function revokePrincipal(id: number) {
  return api.post<PrincipalView>(`/mcp/principals/${id}/revoke`)
}

/** 会话列表 */
export function listMcpSessions(principalId?: number | null, limit = 100) {
  const params: Record<string, number> = { limit }
  if (principalId != null) params.principal_id = principalId
  return api.get<AgentSession[]>('/mcp/sessions', { params })
}

/** 工具调用审计（最新在前） */
export function listToolCalls(query: ToolCallQuery = {}) {
  const params: Record<string, string | number> = {}
  if (query.principal_id != null) params.principal_id = query.principal_id
  if (query.session_id) params.session_id = query.session_id
  if (query.tool) params.tool = query.tool
  if (query.page) params.page = query.page
  if (query.page_size) params.page_size = query.page_size
  return api.get<ToolCallPage>('/mcp/calls', { params })
}

/** 写操作审批单（M-AI-2，最新在前） */
export function listApprovals(query: ApprovalQuery = {}) {
  const params: Record<string, string | number> = {}
  if (query.status) params.status = query.status
  if (query.principal_id != null) params.principal_id = query.principal_id
  if (query.page) params.page = query.page
  if (query.page_size) params.page_size = query.page_size
  return api.get<ApprovalPage>('/mcp/approvals', { params })
}

/** 批准并立即执行；返回终态审批单（succeeded / failed） */
export function approveApproval(id: number) {
  return api.post<ToolApproval>(`/mcp/approvals/${id}/approve`)
}

/** 拒绝（理由回传给 AI） */
export function rejectApproval(id: number, reason = '') {
  return api.post<ToolApproval>(`/mcp/approvals/${id}/reject`, { reason })
}
