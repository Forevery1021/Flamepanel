/**
 * 路由级预取（Doc/30 性能 C）：鼠标悬停 / 键盘聚焦侧栏菜单时提前下载目标视图的 chunk，
 * 点击后不再等待网络。
 *
 * - 只调用路由记录里的懒加载函数（`() => import(...)`），浏览器 / Vite 对同一模块只请求一次；
 * - 悬停 80 ms 才触发（鼠标扫过菜单不会把所有页面都拉下来）；
 * - 省流量模式（Save-Data）或 2G 网络下不预取；
 * - 每个路径只预取一次，失败后允许下次重试。
 */
import type { Router } from 'vue-router'

const HOVER_DELAY_MS = 80

const done = new Set<string>()

interface NetworkInformationLike {
  saveData?: boolean
  effectiveType?: string
}

function constrainedNetwork(): boolean {
  const c = (navigator as Navigator & { connection?: NetworkInformationLike }).connection
  if (!c) return false
  return c.saveData === true || /(^|-)2g$/.test(c.effectiveType ?? '')
}

function isPromise(v: unknown): v is Promise<unknown> {
  return !!v && typeof (v as { then?: unknown }).then === 'function'
}

/** 立即预取 `path` 对应路由的懒加载组件。 */
export function prefetchRoute(router: Router, path: string): void {
  if (!path || done.has(path) || constrainedNetwork()) return
  let matched
  try {
    matched = router.resolve(path).matched
  } catch {
    return
  }
  done.add(path)
  for (const record of matched) {
    for (const comp of Object.values(record.components ?? {})) {
      // 已加载的组件是对象；懒加载项是零参数函数（函数式组件会带 props 参数，跳过）
      if (typeof comp !== 'function' || comp.length > 0) continue
      try {
        const res = (comp as () => unknown)()
        if (isPromise(res)) res.catch(() => done.delete(path))
      } catch {
        done.delete(path)
      }
    }
  }
}

/** 悬停意图：返回 `enter(path)` / `leave()` 处理器，停留超过阈值才预取。 */
export function useHoverPrefetch(router: Router) {
  let timer: ReturnType<typeof setTimeout> | null = null
  const leave = () => {
    if (timer) clearTimeout(timer)
    timer = null
  }
  const enter = (path: string) => {
    leave()
    timer = setTimeout(() => {
      timer = null
      prefetchRoute(router, path)
    }, HOVER_DELAY_MS)
  }
  return { enter, leave, now: (path: string) => prefetchRoute(router, path) }
}

/** 测试用：清空已预取记录。 */
export function resetPrefetchState(): void {
  done.clear()
}
