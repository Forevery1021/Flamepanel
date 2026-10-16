import { computed, ref } from 'vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useI18n } from 'vue-i18n'
import { bindWebsiteRuntime, applyWebsiteRewrite, type SiteType } from '@/api/websites'
import { listRuntimes, listProjects } from '@/api/runtimeProject'
import type { RuntimeDto, ProjectDto } from '@/api/generated'

/** 绑定提交载荷（表单编辑态在弹窗组件内，提交时按此形状上抛） */
export interface WebsiteBindPayload {
  site_type: SiteType
  runtime_id: number | null
  project_id: number | null
  /** 可选的伪静态套用（绑定成功后追加写 vhost，与原实现一致） */
  rewrite_preset?: string
}

/**
 * 站点绑定数据层（Doc/23 §3.3 T2-4；M-B 主线二 Stage A-6 · Doc/20 §5.5）。
 *
 * 绑定弹窗的两个下拉数据源（PHP 运行时 / 项目）此前在 `openBind` 里内联拉取，
 * 与弹窗模板同处一个巨型文件。本 composable 承接「下拉数据加载 + 绑定提交」：
 *
 * - `prepare()`：并行拉取运行时与项目清单，**每次打开都重取**（运维在别处新建的
 *   运行时/项目应当即时可见，原先也是每次打开都请求）；
 * - `bind()`：按站点类型择一上报 `runtime_id` / `project_id`（另一侧显式置 null，
 *   这是后端 `bindWebsiteRuntime` 的语义要求，不是可选优化）；
 * - 选中伪静态模板时，绑定成功后追加一次套用（等价于原 `handleBind` 的第二步）。
 *
 * 下拉数据只经 computed 暴露，编辑态（当前选了哪个运行时/项目）由弹窗本地持有。
 */
export function useWebsiteBinding() {
  const { t } = useI18n()
  const toast = useFpToast()

  const phpRuntimes = ref<RuntimeDto[]>([])
  const projects = ref<ProjectDto[]>([])
  const submitting = ref(false)

  /** 仅可绑定的 PHP 运行时（已安装或运行中）——与原实现过滤条件一致 */
  const phpRuntimeOptions = computed(() =>
    phpRuntimes.value.map((r) => ({ label: `php ${r.version}`, value: r.id })),
  )
  const projectOptions = computed(() =>
    projects.value.map((pr) => ({ label: pr.name, value: pr.id })),
  )

  /** 打开绑定弹窗前调用：拉取两个下拉的数据源 */
  async function prepare() {
    try {
      const [runtimesRes, projectsRes] = await Promise.all([listRuntimes(), listProjects()])
      phpRuntimes.value = (runtimesRes.data || []).filter(
        (r) => r.kind === 'php' && (r.status === 'installed' || r.status === 'running'),
      )
      projects.value = projectsRes.data || []
    } catch {
      toast.error(t('common.failed'))
    }
  }

  /** 提交绑定；成功返回 true（调用方据此关闭弹窗并刷新列表） */
  async function bind(id: number, payload: WebsiteBindPayload): Promise<boolean> {
    submitting.value = true
    try {
      await bindWebsiteRuntime(id, {
        site_type: payload.site_type,
        runtime_id: payload.site_type === 'runtime_php' ? (payload.runtime_id ?? null) : null,
        project_id: payload.site_type === 'reverse_proxy' ? (payload.project_id ?? null) : null,
      })
      if (payload.rewrite_preset) {
        await applyWebsiteRewrite(id, payload.rewrite_preset)
      }
      toast.success(t('common.success'))
      return true
    } catch {
      toast.error(t('common.failed'))
      return false
    } finally {
      submitting.value = false
    }
  }

  return {
    phpRuntimeOptions,
    projectOptions,
    submitting,
    prepare,
    bind,
  }
}
