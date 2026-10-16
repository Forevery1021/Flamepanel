import { ref } from 'vue'
import { listInstalledApps, launchApp } from '@/api/appStore'
import type { InstalledApp } from '@/api/appStore'

/** 常用应用展示上限（原实现直接写在视图里的 `slice(0, 6)`） */
export const COMMON_APP_LIMIT = 6

/**
 * 仪表盘常用应用数据层（Doc/23 §3.3 T2-4 视图拆分第五站）。
 *
 * 两处语义固化为显式事实：
 *
 * 1. **排序在数据层做**：`launch_count` 降序取前 6——「常用」的定义就在这里
 *    （缺失计 0），散在模板里会让排序依据变成一句无人负责的 `sort`。
 * 2. **启动是「尽力而为」**：`launchApp` 失败**不提示**（原实现 `.catch(() => {})`），
 *    因为它的真实目的是记一次启动计数；随后只要 `access_url` 非空就新窗口打开，
 *    打开与否不受计数请求成败影响。计数失败还弹错误，会把一次正常的「打开应用」
 *    体验变成一次报错。
 */
export function useDashboardApps() {
  const apps = ref<InstalledApp[]>([])

  async function fetch() {
    try {
      const res = await listInstalledApps()
      apps.value = [...res.data]
        .sort((a, b) => (b.launch_count ?? 0) - (a.launch_count ?? 0))
        .slice(0, COMMON_APP_LIMIT)
    } catch {
      apps.value = []
    }
  }

  /** 记录一次启动并打开访问地址（计数失败静默；无 access_url 则只计数） */
  function open(app: InstalledApp) {
    launchApp(app.id).catch(() => {})
    if (app.access_url) window.open(app.access_url, '_blank', 'noopener')
  }

  return { apps, fetch, open }
}
