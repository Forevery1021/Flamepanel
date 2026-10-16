import { ref } from 'vue'
import { usePolling } from '@/composables/usePolling'
import { listTopProcesses } from '@/api/metrics'
import type { ProcessEntry } from '@/api/generated'

/** 进程 TOP 轮询间隔（原实现写在视图里的 10s） */
export const PROCESS_POLL_INTERVAL_MS = 10000

/**
 * 仪表盘进程 TOP 数据层（Doc/23 §3.3 T2-4 视图拆分第五站）。
 *
 * 与其余两族（待办 / 常用应用）的差异只在**刷新方式**：进程表走
 * `usePolling(10s)` 而非一次性拉取，且 **`immediate: false`**——
 * 首次数据由视图挂载时的显式 `fetch()` 提供，轮询只负责后续刷新。
 * 若把两者合成 `immediate: true`，首屏会发两次请求（一次 immediate、一次显式）；
 * 这条差异有用例锁死，避免将来「顺手合并」时重复打点。
 *
 * 轮询器本身（`usePolling`）已处理 keep-alive 失活暂停与卸载清理，
 * 故本层不再重复监听生命周期。
 */
export function useDashboardProcesses() {
  const processes = ref<ProcessEntry[]>([])

  async function fetch() {
    try {
      const res = await listTopProcesses()
      processes.value = res.data
    } catch {
      processes.value = []
    }
  }

  const polling = usePolling(fetch, PROCESS_POLL_INTERVAL_MS, { immediate: false })

  return { processes, fetch, startPolling: polling.start, stopPolling: polling.stop }
}
