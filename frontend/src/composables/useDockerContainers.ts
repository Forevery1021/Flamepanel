import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import { useDockerResource } from '@/composables/useDockerResource'
import { filterContainers } from '@/utils/dockerFormat'
import {
  startContainer,
  stopContainer,
  restartContainer,
  removeContainer,
  renameContainer,
  pauseContainer,
  unpauseContainer,
  killContainer,
  pruneContainers,
  containerLogs,
  containerStats,
  inspectContainer,
} from '@/api/docker'
import type { DockerContainer } from '@/types'

/**
 * Docker 容器数据层（Doc/23 §3.3 T2-4 视图拆分第三站）。
 *
 * 原 DockerView（1107 行）把五个页签的表格、弹窗与全部请求编排揉在同一文件。
 * 本 composable 收敛「容器」页签这一族——它是五个页签里操作最密集的一个
 * （九类行操作 + 三个只读弹窗 + 清理），也是拆分后单文件逻辑量最大的落点：
 *
 * - 列表（`useDockerResource`，首个页签默认启用）+ 客户端三字段搜索过滤；
 * - 行操作统一走 `run()` 容错壳：成功 toast + 刷新列表，失败 toast 且不改本地状态；
 * - 重命名（`renaming` 锁提交态，成功返回 true 由调用方关弹窗）；
 * - 日志 / 统计 / 详情三个只读弹窗的数据（打开即拉，失败以占位文案呈现）；
 * - 容器清理（二次确认 → prune → 刷新）。
 *
 * **行操作用「行对象」而非 ID**：模板里九条操作全部直接 `@click="start(row)"`，
 * 若签名收 ID，每个调用点都要写 `$event.id` 这类投影，容易漏改；收行对象后
 * 组件与数据层之间的契约就是「哪一行」这一个概念。
 *
 * 行为与原实现逐行对齐（含错误文案、失败后不刷新、`9999` 行哨兵），无契约变更。
 */
export function useDockerContainers() {
  const { t } = useI18n()
  const toast = useFpToast()
  const { confirmAction } = useFpConfirm()

  /** Y3：首个页签默认启用，其余页签由视图层首次切换时置 true */
  const enabled = ref(true)
  const res = useDockerResource('containers', enabled)

  const search = ref('')
  const filtered = ref<DockerContainer[]>([])

  watch(
    [res.data, search],
    () => {
      filtered.value = filterContainers(res.data.value ?? [], search.value)
    },
    { immediate: true },
  )

  /** 统一写操作容错壳：成功刷新 + toast，失败 toast（不静默、不改本地状态） */
  async function run(fn: () => Promise<unknown>): Promise<boolean> {
    try {
      await fn()
      toast.success(t('common.success'))
      await res.refresh()
      return true
    } catch (e) {
      toast.error(e, t('common.failed'))
      return false
    }
  }

  /* ── 行操作（无确认的三类：启停/重启/强停） ── */

  const start = (row: DockerContainer) => run(() => startContainer(row.id))
  const stop = (row: DockerContainer) => run(() => stopContainer(row.id))
  const restart = (row: DockerContainer) => run(() => restartContainer(row.id))
  const kill = (row: DockerContainer) => run(() => killContainer(row.id))

  /** 删除带二次确认（原 `confirmRemoveContainer`） */
  function remove(row: DockerContainer) {
    confirmAction({
      message: t('docker.removeConfirm', { name: row.name || row.id }),
      header: t('common.confirmAction'),
      accept: () => run(() => removeContainer(row.id)),
    })
  }

  /** 暂停 / 恢复：按钮承载双向语义，按当前状态择一（原 `togglePause`） */
  function togglePause(row: DockerContainer) {
    return run(() => (row.status === 'paused' ? unpauseContainer(row.id) : pauseContainer(row.id)))
  }

  /* ── 重命名 ── */

  const renaming = ref(false)

  /** 提交重命名；成功返回 true（调用方据此关闭弹窗） */
  async function rename(id: string, name: string): Promise<boolean> {
    renaming.value = true
    try {
      await renameContainer(id, name.trim())
      toast.success(t('common.success'))
      await res.refresh()
      return true
    } catch (e) {
      toast.error(e, t('common.failed'))
      return false
    } finally {
      renaming.value = false
    }
  }

  /* ── 三个只读弹窗的数据 ── */

  const logsContent = ref('')
  const statsData = ref<unknown>(null)
  const detailContent = ref('')

  /**
   * 日志：`target` 为容器名（回退 ID）——与原实现一致，Docker 的名称与 ID 均可寻址；
   * `tail` 为行数（9999 表示尽量多），失败回落失败文案
   */
  async function loadLogs(target: string, tail: number) {
    try {
      const r = await containerLogs(target, tail)
      logsContent.value = r.data
    } catch {
      logsContent.value = t('common.failed')
    }
  }

  /** 统计：失败置空（弹窗显示「暂无统计数据」占位） */
  async function loadStats(id: string) {
    try {
      const r = await containerStats(id)
      statsData.value = r.data
    } catch {
      statsData.value = null
    }
  }

  /** 详情：打开即显示「加载中」，失败显示失败文案 */
  async function loadDetail(id: string) {
    detailContent.value = t('common.loading')
    try {
      const r = await inspectContainer(id)
      detailContent.value = JSON.stringify(r.data, null, 2)
    } catch {
      detailContent.value = t('common.failed')
    }
  }

  /* ── 清理 ── */

  function prune() {
    confirmAction({
      message: t('docker.pruneConfirm', { what: t('docker.containers') }),
      header: t('common.confirmAction'),
      accept: async () => {
        try {
          await pruneContainers()
          toast.success(t('common.success'))
          await res.refresh()
        } catch (e) {
          toast.error(e, t('common.failed'))
        }
      },
    })
  }

  return {
    containers: res.data,
    filtered,
    loading: res.loading,
    error: res.error,
    search,
    fetch: res.refresh,
    enabled,
    start,
    stop,
    restart,
    remove,
    kill,
    togglePause,
    renaming,
    rename,
    logsContent,
    statsData,
    detailContent,
    loadLogs,
    loadStats,
    loadDetail,
    prune,
  }
}
