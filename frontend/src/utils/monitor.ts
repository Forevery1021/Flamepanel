/**
 * 前端监控接入点（P7 可靠性）。
 *
 * 本期仅做结构化日志 + 统一的全局错误上报入口 `reportError`，
 * 为后续接入 Sentry 或自建监控预留单点，不引入 SDK。
 */

export interface ReportErrorInfo {
  /** 错误来源：render（渲染）/ query（查询）/ mutation（写操作）/ unhandled（未捕获） */
  source?: 'render' | 'query' | 'mutation' | 'unhandled'
  /** 附加上下文（组件名、queryKey 等） */
  context?: unknown
}

/**
 * 全局错误上报单点。
 * 当前实现：结构化 console 输出（开发环境带堆栈，生产环境精简）。
 * 后续接入 Sentry：`Sentry.captureException(error, { extra: info })`。
 */
export function reportError(error: unknown, info: ReportErrorInfo = {}) {
  const { source = 'unhandled', context } = info
  console.error(`[flamepanel:${source}]`, {
    message: error instanceof Error ? error.message : String(error),
    ...(error instanceof Error && error.stack ? { stack: error.stack } : {}),
    ...(context !== undefined ? { context } : {}),
  })
}
