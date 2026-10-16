/**
 * WAF 基础防护 API（主线三 §6.2 · /api/waf）。
 * 类型来自 OpenAPI 生成（src/api/generated），禁止手写重复定义。
 * 规则同时编译下发为 Nginx / OpenResty 原生配置（`applyWafRules`）；日志只含来源 IP / 请求行摘要。
 */
import api from './client'
import type {
  WafRule,
  WafLog,
  WafEngineStatus,
  WafApplyReport,
  CreateWafRuleRequest,
  UpdateWafRuleRequest,
} from '@/api/generated'

export interface WafLogPage {
  data: WafLog[]
  page: number
  page_size: number
  total: number
  total_pages: number
}

export function listWafRules(websiteId = 0) {
  return api.get<WafRule[]>('/waf/rules', { params: { website_id: websiteId } })
}

export function getWafRule(id: number) {
  return api.get<WafRule>(`/waf/rules/${id}`)
}

export function createWafRule(input: CreateWafRuleRequest) {
  return api.post<WafRule>('/waf/rules', input)
}

export function updateWafRule(id: number, input: UpdateWafRuleRequest) {
  return api.put<WafRule>(`/waf/rules/${id}`, input)
}

export function deleteWafRule(id: number) {
  return api.delete<void>(`/waf/rules/${id}`)
}

export function listWafLogs(page: number, pageSize: number, websiteId = 0) {
  return api.get<WafLogPage>('/waf/logs', {
    params: { page, page_size: pageSize, website_id: websiteId },
  })
}

/** 清理拦截日志（websiteId = 0 清空全部），返回清理条数 */
export function clearWafLogs(websiteId = 0) {
  return api.post<number>('/waf/logs/clear', { website_id: websiteId })
}

export function getWafStatus() {
  return api.get<WafEngineStatus>('/waf/status')
}

/** 下发规则到本机 Web 引擎（Nginx / OpenResty），返回逐引擎报告；未配置下发时 503 */
export function applyWafRules() {
  return api.post<WafApplyReport>('/waf/apply')
}
