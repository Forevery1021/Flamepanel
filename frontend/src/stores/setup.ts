import { defineStore } from 'pinia'
import { ref } from 'vue'
import { getSetupStatus, initializeSetup, type SetupStatus } from '@/api/setup'

export const useSetupStore = defineStore('setup', () => {
  const status = ref<SetupStatus | null>(null)
  const statusLoaded = ref(false)

  /** 单飞去重：多个守卫/组件并发请求只发一次 */
  let inflight: Promise<SetupStatus | null> | null = null

  async function fetchStatus(): Promise<SetupStatus | null> {
    if (inflight) return inflight
    inflight = getSetupStatus()
      .then((res) => {
        status.value = res.data
        statusLoaded.value = true
        return res.data
      })
      .catch(() => {
        // 网络失败时保留旧值；守卫侧按“未知”放行登录页，避免安装场景白屏
        return status.value
      })
      .finally(() => {
        inflight = null
      })
    return inflight
  }

  async function ensureStatus(): Promise<SetupStatus | null> {
    if (statusLoaded.value) return status.value
    return fetchStatus()
  }

  /**
   * 向导最终提交（database 步 + admin 步），返回初始化响应。
   *
   * T1-2 缺陷修复（浏览器实测）：**写路径必须同步状态缓存**。
   * 向导期间 `ensureStatus()` 已把 `in_progress` 缓存（`statusLoaded`），若提交完成后
   * 不更新它，`router.push('/dashboard')` 会被守卫用**过期缓存**判回 `/setup`——
   * 用户提交完初始化却停留在向导页，且刷新不自愈（`/setup` 路由对所有访问直接放行、
   * 不再探测状态），必须手工改 URL 才能进入面板。
   *
   * 终态直接以**后端响应**为准落缓存（而非置脏重探）：万一随后的复探因网络失败，
   * `fetchStatus()` 的离线兜底会回落到 `status` 旧值——若不落盘就会再次拦回向导。
   */
  async function complete(payload: Parameters<typeof initializeSetup>[0]) {
    const res = await initializeSetup(payload)
    const next = res.data?.status
    if (next === 'completed') {
      status.value = {
        status: next,
        theme: status.value?.theme ?? 'flame',
        language: status.value?.language ?? 'zh-CN',
        docker: status.value?.docker ?? null,
        nginx: status.value?.nginx ?? null,
      }
      statusLoaded.value = true
    }
    return res.data
  }

  return { status, statusLoaded, fetchStatus, ensureStatus, complete }
})
