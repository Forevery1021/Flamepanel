/**
 * 前端监控接入点（P7 可靠性）。
 *
 * 本期提供结构化日志 + 统一的全局错误上报入口 `reportError`，
 * 并内置去重/节流缓冲，为后续接入 Sentry 或自建上报预留单点。
 *
 * 说明：后端 `/logs` 当前为只读（list/delete），暂无前端上报端点，
 * 故本轮仅做前端侧采集与缓冲；待后端提供上报接口后，可在 `flushErrors`
 * 处统一批量 POST（B5）。
 *
 * R-3（Doc/23 §2.5）：**首次出现的错误必须立即输出**，去重只作用于之后的重复错误。
 * 早期实现把「登记」与「输出」合并进一条提前 return 分支，导致每个 source+message
 * 的第一次错误被静默吞掉——而首错恰恰是排障最需要的一条（后续重复只是它的回声）。
 */

export interface ReportErrorInfo {
  /** 错误来源：render（渲染）/ query（查询）/ mutation（写操作）/ unhandled（未捕获） */
  source?: 'render' | 'query' | 'mutation' | 'unhandled'
  /** 附加上下文（组件名、queryKey 等） */
  context?: unknown
}

/** B5：按 source 去重 + 节流缓冲，避免同一错误高频刷屏/重复上报 */
const recent: Map<string, { count: number; ts: number }> = new Map()
const DEDUP_WINDOW_MS = 30_000

/**
 * 全局错误上报单点。
 * 结构化输出；**首错立即输出**，其后相同 source+message 在去重窗口内节流
 * （只累计计数，窗口外再次输出时携带累计数）。
 * 后续接入 Sentry：`Sentry.captureException(error, { extra: info })`。
 */
export function reportError(error: unknown, info: ReportErrorInfo = {}) {
  const { source = 'unhandled', context } = info
  const message = error instanceof Error ? error.message : String(error)
  const key = `${source}:${message}`
  const now = Date.now()
  const prev = recent.get(key)

  let count = 1
  if (prev) {
    prev.count += 1
    // 重复错误：窗口内静默节流（首错已在首次调用时输出）
    if (now - prev.ts < DEDUP_WINDOW_MS) return
    prev.ts = now
    count = prev.count
  } else {
    // R-3：首错立即登记并输出——不做提前 return，避免排障时第一条线索被吞
    recent.set(key, { count: 1, ts: now })
  }

  console.error(`[flamepanel:${source}]`, {
    message,
    count,
    ...(error instanceof Error && error.stack ? { stack: error.stack } : {}),
    ...(context !== undefined ? { context } : {}),
  })
}

/**
 * B5：上报缓冲刷新入口。后端提供上报接口后，在此批量发送近期采集到的错误；
 * 当前仅清空缓冲（避免内存累积），为后续接入预留。
 */
export function flushErrors(): void {
  recent.clear()
}
