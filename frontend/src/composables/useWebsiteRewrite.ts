import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import {
  applyWebsiteRewrite,
  getWebsiteRewrite,
  clearWebsiteRewrite,
  REWRITE_PRESETS,
  type SiteRewriteStatus,
} from '@/api/websites'

/**
 * 站点伪静态套用数据层（Doc/23 §3.3 T2-4；Doc/20 §5.5 收口）。
 *
 * 与原实现一致的语义：套用即写 vhost 生效，可清除回落默认路由。三处细节值得留档：
 *
 * 1. **模板清单以接口 `supported` 为权威**（后端是模板库的单一事实来源），
 *    接口未返回时回落本地常量 `REWRITE_PRESETS`——两者任一变化都不会让下拉变空；
 * 2. 每次写操作后**重取一次状态**再关弹窗（`rules` 预览要用服务端生成的真实文本）；
 * 3. 编辑态（当前选中的模板名）由弹窗组件本地持有，此处只暴露服务端状态。
 */
export function useWebsiteRewrite() {
  const { t } = useI18n()
  const toast = useFpToast()

  const status = ref<SiteRewriteStatus>({ preset: null, rules: '', supported: [] })
  const submitting = ref(false)

  /** 下拉选项：优先接口 supported，未加载时回落本地常量 */
  const presetOptions = computed(() => {
    const names = status.value.supported.length ? status.value.supported : [...REWRITE_PRESETS]
    return names.map((n) => ({ label: n, value: n }))
  })

  function loadStatus(s: SiteRewriteStatus) {
    status.value = { preset: s.preset ?? null, rules: s.rules ?? '', supported: s.supported ?? [] }
  }

  /** 打开弹窗前调用：拉取该站点的当前套用状态；成功返回状态（失败返回 null，弹窗仍可打开） */
  async function prepare(id: number): Promise<SiteRewriteStatus | null> {
    try {
      const res = await getWebsiteRewrite(id)
      loadStatus(res.data)
      return res.data
    } catch {
      toast.error(t('common.failed'))
      return null
    }
  }

  /** 套用模板；成功返回 true */
  async function apply(id: number, preset: string): Promise<boolean> {
    if (!preset) return false
    submitting.value = true
    try {
      await applyWebsiteRewrite(id, preset)
      const res = await getWebsiteRewrite(id)
      loadStatus(res.data)
      toast.success(t('common.success'))
      return true
    } catch (e) {
      toast.error(e, t('common.failed'))
      return false
    } finally {
      submitting.value = false
    }
  }

  /** 清除套用；成功返回 true */
  async function clear(id: number): Promise<boolean> {
    submitting.value = true
    try {
      const res = await clearWebsiteRewrite(id)
      loadStatus(res.data)
      toast.success(t('common.success'))
      return true
    } catch (e) {
      toast.error(e, t('common.failed'))
      return false
    } finally {
      submitting.value = false
    }
  }

  return {
    status,
    presetOptions,
    submitting,
    prepare,
    apply,
    clear,
  }
}
