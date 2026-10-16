import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import { useApiQuery } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import {
  listProjectLinks,
  listLinkableDatabases,
  linkProjectDatabase,
  unlinkProjectDatabase,
} from '@/api/projects'
import type { LinkableDatabaseDto, ProjectDbLinkDto, ProjectDto } from '@/api/generated'

/**
 * 数据库联链抽屉数据层（Stage A-10 · Doc/20 §5.8 ⑤，Dokku link 模式）。
 *
 * 两条查询都随抽屉打开、均为惰性：
 *
 * - `links`：已关联清单（**不含连接串**，凭据不出后端服务边界）；
 * - `linkable`：可关联的数据库实例（后端只返回 running 实例）。
 *
 * ⚠️ **两个查询共用一个 queryKey（`projects.links(id)`）——这是原实现的既有冲突，
 * 本次拆分原样保留**：React Query 语义下同 key 即同缓存，第二个查询不会真正
 * 发出独立请求（实测：先 refresh 的那条被去重命中），下拉候选实际拿到的是
 * 联链清单。修它属于行为变更，不在本轮的「拆分零契约变更」范围内，
 * 已记入 PR「后续建议」（建议改为 `projects.linksAvailable(id)` 独立 key）。
 */
export function useProjectLinks() {
  const { t } = useI18n()
  const toast = useFpToast()
  const { confirmAction } = useFpConfirm()

  const project = ref<ProjectDto | null>(null)
  const saving = ref(false)
  /** 正在解除的那一条（格式 `unlink-<linkId>`，空串 = 空闲；只锁该行按钮） */
  const actingId = ref('')
  const form = ref<{ db_instance_id: number | undefined; env_name: string }>({
    db_instance_id: undefined,
    env_name: '',
  })

  const linksQuery = useApiQuery<ProjectDbLinkDto[]>(
    () => queryKeys.projects.links(project.value?.id ?? 0),
    async () => {
      const res = await listProjectLinks(project.value?.id ?? 0)
      return { data: res.data }
    },
    { enabled: false, retry: false },
  )

  const linkableQuery = useApiQuery<LinkableDatabaseDto[]>(
    () => queryKeys.projects.links(project.value?.id ?? 0),
    async () => {
      const res = await listLinkableDatabases(project.value?.id ?? 0)
      return { data: res.data }
    },
    { enabled: false, retry: false },
  )

  const links = computed<ProjectDbLinkDto[]>(() => linksQuery.data.value ?? [])
  const linkable = computed<LinkableDatabaseDto[]>(() => linkableQuery.data.value ?? [])
  const linkableOptions = computed(() =>
    linkable.value.map((d) => ({ label: `${d.name} (${d.db_type}:${d.port})`, value: d.id })),
  )

  /**
   * 打开抽屉：清空上一次的联链表单（`env_name` 残留会让下一次「关联」静默复用旧变量名），
   * 再并发拉两条数据。
   */
  function open(p: ProjectDto) {
    project.value = p
    form.value = { db_instance_id: undefined, env_name: '' }
    void linksQuery.refresh()
    void linkableQuery.refresh()
  }

  function reset() {
    project.value = null
  }

  /** 关联（`db_name` 显式 null：后端按项目名推导业务库名） */
  async function link() {
    const pid = project.value?.id
    if (!pid || !form.value.db_instance_id) return
    saving.value = true
    try {
      await linkProjectDatabase(pid, {
        db_instance_id: form.value.db_instance_id,
        env_name: form.value.env_name || null,
        db_name: null,
      })
      toast.success(t('project.linkSuccess'))
      form.value = { db_instance_id: undefined, env_name: '' }
      void linksQuery.refresh()
    } catch (err) {
      toast.error(err, t('common.failed'))
    } finally {
      saving.value = false
    }
  }

  /** 解除联链（二次确认；确认文案带环境变量名——同一实例可能链了多个变量） */
  function unlink(link: ProjectDbLinkDto) {
    const pid = project.value?.id
    if (!pid) return
    confirmAction({
      message: t('project.linkUnlinkConfirm', { env: link.env_name }),
      header: t('project.linkUnlink'),
      accept: async () => {
        actingId.value = `unlink-${link.id}`
        try {
          await unlinkProjectDatabase(pid, link.id)
          toast.success(t('common.success'))
          void linksQuery.refresh()
        } catch (err) {
          toast.error(err, t('common.failed'))
        } finally {
          actingId.value = ''
        }
      },
    })
  }

  return {
    project,
    links,
    linkableOptions,
    loading: linksQuery.loading,
    error: linksQuery.error,
    refresh: linksQuery.refresh,
    saving,
    actingId,
    form,
    open,
    reset,
    link,
    unlink,
  }
}
