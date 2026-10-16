import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import { getErrorDetail } from '@/utils/error'
import { getStoreSecurity, updateStoreSecurity, type StoreSecurityConfig } from '@/api/appStore'

/**
 * 商店安全配置的服务端状态（Doc/20 §4.2 收口；Doc/23 §3.3 T2-4 视图拆分）。
 *
 * 职责边界刻意为「服务端状态 + 读写编排」，**不含编辑态**——验签策略与白名单是
 * 「本地暂存 → 整体提交」的编辑会话（`registry_whitelist` 为全量替换语义），
 * 编辑态归展示层 `StoreSecurityPanel` 持有，否则组件必须 mutate 传入对象
 * （`vue/no-mutating-props` 也正是这么要求的）。
 *
 * 懒加载：安全页签首次激活才请求（避免首屏多打一次配置接口）。
 */
export function useAppStoreSecurity() {
  const { t } = useI18n()
  const toast = useFpToast()

  const loading = ref(false)
  const saving = ref(false)
  const config = ref<StoreSecurityConfig | null>(null)

  async function load() {
    loading.value = true
    try {
      const res = await getStoreSecurity()
      config.value = res.data
    } catch (e: unknown) {
      toast.error(e, getErrorDetail(e) || t('common.failed'))
    } finally {
      loading.value = false
    }
  }

  /** 懒加载入口：首次激活时取配置；已在请求中或已加载则跳过 */
  function activate() {
    if (!config.value && !loading.value) load()
  }

  /** 保存（成功回写 config，供展示层同步编辑态与内置清单只读区） */
  async function save(payload: {
    signature_policy: string
    registry_whitelist: string[]
  }): Promise<boolean> {
    saving.value = true
    try {
      const res = await updateStoreSecurity(payload)
      config.value = res.data
      toast.success(t('appStore.securitySaved'))
      return true
    } catch (e: unknown) {
      toast.error(e, getErrorDetail(e) || t('common.failed'))
      return false
    } finally {
      saving.value = false
    }
  }

  return { loading, saving, config, load, activate, save }
}
