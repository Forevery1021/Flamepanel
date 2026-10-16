import { computed, ref, watch } from 'vue'
import { useApiQuery } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import { getHostDetails, type HostDetails } from '@/api/toolbox'
import { clockSkewSeconds } from '@/utils/hostInfo'

/**
 * 系统工具箱 · 主机详情数据层：`GET /api/toolbox/host`。
 *
 * 主机详情变化很慢，30 s 内重复进入页面不重复请求；手动刷新走 `refresh()`。
 * 每次拿到新数据时记录浏览器接收时刻，用来估算服务器时钟偏差。
 */
export function useHostDetails() {
  const query = useApiQuery<HostDetails>(queryKeys.toolbox.host(), getHostDetails, {
    staleTime: 30_000,
    retry: 1,
  })

  const receivedAt = ref(Date.now())
  watch(
    () => query.data.value,
    (v) => {
      if (v) receivedAt.value = Date.now()
    },
    { immediate: true },
  )

  const host = computed(() => query.data.value ?? null)
  const skewSecs = computed(() =>
    host.value ? clockSkewSeconds(host.value.time.now, receivedAt.value) : 0,
  )

  return {
    host,
    skewSecs,
    loading: query.loading,
    error: query.error,
    refresh: query.refresh,
  }
}
