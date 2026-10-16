import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import { useApiQuery } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import { listProjects, createProject, updateProject } from '@/api/projects'
import { listRuntimes } from '@/api/runtimes'
import type { ProjectDto, RuntimeDto } from '@/api/generated'
import {
  buildProjectBody,
  buildProjectCreateBody,
  runtimeOptions,
  type ProjectFormState,
} from '@/utils/projectFormat'

/**
 * 项目列表数据层（Doc/23 §3.3 T2-4 视图拆分第六站）。
 *
 * 原 ProjectView（832 行）把「列表加载 + 客户端搜索 + 增删改 + 六类生命周期动作 +
 * Webhook 令牌 + 日志抽屉 + 数据库联链」全部塞在同一文件。本 composable 收敛
 * 列表族（另见 `useProjectLifecycle` / `useProjectLogs` / `useProjectLinks`）：
 *
 * - 列表查询（`useApiQuery`，`queryKeys.projects.list`）与刷新；
 * - 客户端搜索（**名称 + 域名**两字段包含匹配，原实现即如此——按根路径搜不在口径内）；
 * - 创建 / 编辑（表单编辑态归弹窗组件，此处只收「已是合法表单」的载荷并组装请求体）；
 * - 运行时清单（创建弹窗的绑定下拉数据源，**按 kind 去重**见 `utils/projectFormat`）。
 *
 * 写操作不做本地校验：空值归一（`'' → null`）在 `buildProjectBody` 里，弹窗侧的
 * 本地校验只负责「字段旁的红字」，两者不重复。
 */
export function useProjectList() {
  const { t } = useI18n()
  const toast = useFpToast()

  const search = ref('')
  const submitting = ref(false)

  const projectsQuery = useApiQuery<ProjectDto[]>(
    () => queryKeys.projects.list(),
    async () => {
      const res = await listProjects()
      return { data: res.data }
    },
  )
  const projects = computed<ProjectDto[]>(() => projectsQuery.data.value ?? [])
  const loading = projectsQuery.loading

  const runtimesQuery = useApiQuery<RuntimeDto[]>(
    () => queryKeys.runtimes.list(),
    async () => {
      const res = await listRuntimes()
      return { data: res.data }
    },
  )

  /**
   * 客户端搜索：名称 / 域名两字段（大小写不敏感；空查询返回原数组）。
   *
   * 与列表同处一层而非展示层：过滤口径是「哪些行可见」的数据语义，
   * 拆出后可以被单测锁死（原实现是视图里的内联 computed，测不到）。
   */
  const filteredProjects = computed<ProjectDto[]>(() => {
    const kw = search.value.trim().toLowerCase()
    if (!kw) return projects.value
    return projects.value.filter(
      (p) => p.name.toLowerCase().includes(kw) || (p.domain ?? '').toLowerCase().includes(kw),
    )
  })

  /** 运行时下拉候选（同 kind 只留首个版本——选项爆炸防线，见纯函数层） */
  const runtimeSelectOptions = computed(() => runtimeOptions(runtimesQuery.data.value ?? []))

  function fetch() {
    return projectsQuery.refresh()
  }

  /** 新建；成功返回 true（调用方据此关闭弹窗） */
  async function create(form: ProjectFormState): Promise<boolean> {
    submitting.value = true
    try {
      await createProject(buildProjectCreateBody(form))
      toast.success(t('project.createSuccess'))
      await fetch()
      return true
    } catch (err) {
      toast.error(err, t('common.failed'))
      return false
    } finally {
      submitting.value = false
    }
  }

  /** 编辑保存；成功返回 true（`name` 不可改——它是部署目录名，故不进请求体） */
  async function update(id: number, form: ProjectFormState): Promise<boolean> {
    submitting.value = true
    try {
      await updateProject(id, buildProjectBody(form))
      toast.success(t('project.updateSuccess'))
      await fetch()
      return true
    } catch (err) {
      toast.error(err, t('common.failed'))
      return false
    } finally {
      submitting.value = false
    }
  }

  return {
    projects,
    filteredProjects,
    loading,
    search,
    fetch,
    submitting,
    create,
    update,
    runtimeSelectOptions,
  }
}
