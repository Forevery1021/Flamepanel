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

  /** 向导最终提交（database 步 + admin 步），返回初始化响应 */
  async function complete(
    payload: Parameters<typeof initializeSetup>[0],
  ) {
    const res = await initializeSetup(payload)
    return res.data
  }

  return { status, statusLoaded, fetchStatus, ensureStatus, complete }
})