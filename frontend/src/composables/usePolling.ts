import { onBeforeUnmount, onDeactivated, onActivated } from 'vue'

export interface UsePolling {
  /** 启动轮询（幂等） */
  start: () => void
  /** 停止轮询 */
  stop: () => void
  /** 暂停（keep-alive 失活自动调用） */
  pause: () => void
  /** 恢复（keep-alive 激活自动调用） */
  resume: () => void
  /** 是否处于激活态 */
  isActive: () => boolean
}

/**
 * usePolling — 统一定时器轮询生命周期管理。
 * - 组件卸载自动 stop（clearInterval）
 * - keep-alive 失活（onDeactivated）自动 pause，恢复（onActivated）自动 resume
 * - 配合页面可见性：可在回调内自行判断 document.visibilityState
 */
export function usePolling(fn: () => void, intervalMs: number, options: { immediate?: boolean } = {}): UsePolling {
  let timer: number | null = null
  let active = true
  const { immediate = false } = options

  function start() {
    if (!active || timer !== null) return
    if (immediate) fn()
    timer = window.setInterval(fn, intervalMs)
  }

  function stop() {
    if (timer !== null) {
      window.clearInterval(timer)
      timer = null
    }
  }

  function pause() {
    active = false
    stop()
  }

  function resume() {
    active = true
    start()
  }

  function isActive() {
    return active
  }

  onActivated(() => resume())
  onDeactivated(() => pause())
  onBeforeUnmount(() => stop())

  return { start, stop, pause, resume, isActive }
}
