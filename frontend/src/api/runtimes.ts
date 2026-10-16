/**
 * Runtime 运行时 API（M-B 主线二 Stage A-5 · Doc/20 §5.6/5.7）。
 *
 * 类型来自 OpenAPI 生成单源（src/api/generated，见 PR #45 后端契约）。
 */
import api from './client'
import type { RuntimeDto, RuntimeVersionInfo, InstallRuntimeRequest } from '@/api/generated'

/** 语言运行时种类（与后端 Runtime::is_valid_kind 白名单对齐） */
export const RUNTIME_KINDS = ['php', 'java', 'node', 'python', 'go', 'tomcat'] as const
export type RuntimeKind = (typeof RUNTIME_KINDS)[number]

/** 运行时列表 */
export function listRuntimes() {
  return api.get<RuntimeDto[]>('/runtimes')
}

/** 按语言过滤运行时 */
export function listRuntimesByKind(kind: RuntimeKind) {
  return api.get<RuntimeDto[]>(`/runtimes/kind/${kind}`)
}

/** 可安装版本清单（声明版本 × 系统探测状态） */
export function availableRuntimes(kind: RuntimeKind) {
  return api.get<RuntimeVersionInfo[]>('/runtimes/available', {
    params: { kind },
  })
}

/** 安装运行时（登记 + 安装，耗时操作走 longApi 语义——不设超时由任务轮询反馈） */
export function installRuntime(req: InstallRuntimeRequest) {
  return api.post<RuntimeDto>('/runtimes/install', req, { timeout: 0 })
}

/** 卸载运行时 */
export function uninstallRuntime(id: number) {
  return api.delete<{ ok: boolean }>(`/runtimes/${id}`)
}

/** 设为同语言默认版本 */
export function setDefaultRuntime(id: number) {
  return api.post<RuntimeDto>(`/runtimes/${id}/default`)
}
