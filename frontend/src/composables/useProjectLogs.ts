import { computed, onBeforeUnmount, ref } from 'vue'
import { useApiQuery } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import { getProjectLogs } from '@/api/projects'
import { formatLogLines } from '@/utils/projectFormat'
import type { LogLine, ProjectDto } from '@/api/generated'

/**
 * 项目日志抽屉数据层（Doc/23 §3.3 T2-4 视图拆分第六站）。
 *
 * 日志是**惰性查询**（`enabled: false` + 打开时显式 `refresh()`）：
 * 列表里每行都有「日志」按钮，若跟着列表一起拉，进入页面就会对每个项目
 * 各打一次 `/logs`（后端的 tail 是 200 行，代价不低）。
 *
 * 抽屉关闭用 `reset()` 而不是「留着上次数据」：下次打开的是另一个项目，
 * 残留的旧日志会先渲染一屏再被替换（视觉上像「日志变了」）；同时
 * `onBeforeUnmount` 也得收口——原实现在视图卸载时把 `showLogs` 置 false，
 * 本层等价地把持有目标清空（组件卸载后回调不该再写 ref）。
 */
export function useProjectLogs() {
  const project = ref<ProjectDto | null>(null)

  const logsQuery = useApiQuery<LogLine[]>(
    () => queryKeys.projects.logs(project.value?.id ?? 0),
    async () => {
      const res = await getProjectLogs(project.value?.id ?? 0)
      return { data: res.data }
    },
    { enabled: false, retry: false },
  )

  const lines = computed<LogLine[]>(() => logsQuery.data.value ?? [])
  const text = computed(() => formatLogLines(lines.value))

  /** 打开抽屉：先记目标再拉日志（queryKey 依赖目标 id） */
  function open(p: ProjectDto) {
    project.value = p
    void logsQuery.refresh()
  }

  /** 抽屉关闭：清空目标（再次打开时是新项目，不残留旧日志） */
  function reset() {
    project.value = null
  }

  onBeforeUnmount(reset)

  return {
    project,
    lines,
    text,
    /** 首屏三态（状态面板用：无缓存时才切骨架） */
    loading: logsQuery.loading,
    /** 任意一次拉取中（重拉按钮用：已有日志时刷新只锁按钮，不闪骨架） */
    fetching: logsQuery.isFetching,
    error: logsQuery.error,
    refresh: logsQuery.refresh,
    open,
    reset,
  }
}
