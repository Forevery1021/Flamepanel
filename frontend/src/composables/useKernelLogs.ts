import { computed, onActivated, onBeforeUnmount, onDeactivated, ref, shallowRef, watch } from 'vue'
import {
  LOG_LEVELS,
  deleteKernelLogFile,
  downloadKernelLogFile,
  getKernelLogOverview,
  getKernelLogStats,
  listKernelLogEntries,
  setKernelLogLevel,
  type KernelLogEntry,
  type KernelLogOverview,
  type LevelCounts,
  type LogLevel,
} from '@/api/kernelLogs'

/** 每页条数（后端上限 1000） */
export const PAGE_SIZE = 200
/** 跟随模式轮询间隔（毫秒） */
export const FOLLOW_INTERVAL_MS = 3000
/** 关键字防抖（毫秒） */
export const SEARCH_DEBOUNCE_MS = 300
/** 跟随模式下最多保留的行数（防止长时间挂着页面内存上涨） */
export const FOLLOW_MAX_ROWS = 2000

export interface KernelLogRow extends KernelLogEntry {
  /** 前端稳定 key（时间戳 + 列表序号，同一时间戳多行时靠序号区分） */
  key: string
}

function withKeys(entries: KernelLogEntry[], offset: number): KernelLogRow[] {
  return entries.map((e, i) => ({ ...e, key: `${e.timestamp}#${offset + i}` }))
}

/**
 * 内核日志面板状态机：
 * - 文件 / 级别 / 关键字任一变化 → 重新拉第一页（关键字防抖）
 * - 「加载更早」沿 `next_before` 游标续读并追加
 * - 跟随模式：定时检查文件大小，变化才重拉第一页；与已加载的旧页去重合并
 */
export function useKernelLogs(initial: { q?: string } = {}) {
  const overview = shallowRef<KernelLogOverview | null>(null)
  const file = ref<string | undefined>(undefined)
  const levels = ref<LogLevel[]>([])
  const keyword = ref(initial.q ?? '')
  const rows = shallowRef<KernelLogRow[]>([])
  const nextBefore = ref<number | null>(null)
  const counts = shallowRef<LevelCounts | null>(null)
  const countsTruncated = ref(false)
  const loading = ref(false)
  const loadingMore = ref(false)
  const error = shallowRef<unknown>(null)
  const following = ref(false)
  /** 本次跟随期间新增的行 key（行首短暂高亮用） */
  const freshKeys = shallowRef<Set<string>>(new Set())

  let lastFileSize = -1
  let requestSeq = 0
  let followTimer: ReturnType<typeof setInterval> | null = null
  let searchTimer: ReturnType<typeof setTimeout> | null = null

  const enabled = computed(() => overview.value?.enabled ?? false)
  const files = computed(() => overview.value?.files ?? [])
  const currentFile = computed(() => files.value.find((f) => f.current)?.name)
  const isViewingCurrent = computed(() => !file.value || file.value === currentFile.value)
  const hasMore = computed(() => nextBefore.value != null)

  async function loadOverview() {
    const { data } = await getKernelLogOverview()
    overview.value = data
    if (file.value && !data.files.some((f) => f.name === file.value)) {
      file.value = undefined
    }
    if (!file.value) file.value = data.files.find((f) => f.current)?.name ?? data.files[0]?.name
  }

  async function loadStats() {
    if (!file.value) {
      counts.value = null
      return
    }
    try {
      const { data } = await getKernelLogStats(file.value)
      counts.value = data.counts
      countsTruncated.value = data.truncated
    } catch {
      // 计数只是辅助信息，失败不打断列表
      counts.value = null
    }
  }

  /** 重拉第一页；`merge` 为跟随模式：只把新行合并到头部 */
  async function reload(opts: { merge?: boolean } = {}) {
    if (!enabled.value || !file.value) {
      rows.value = []
      nextBefore.value = null
      return
    }
    const seq = ++requestSeq
    if (!opts.merge) loading.value = true
    try {
      const { data } = await listKernelLogEntries({
        file: file.value,
        levels: levels.value,
        q: keyword.value,
        limit: PAGE_SIZE,
      })
      if (seq !== requestSeq) return // 过期响应（用户已切换条件）
      error.value = null
      lastFileSize = data.file_size
      if (opts.merge && rows.value.length) {
        const newest = rows.value[0]
        const idx = data.entries.findIndex(
          (e) => e.timestamp === newest.timestamp && e.message === newest.message,
        )
        // 找不到接缝（新增超过一页）→ 直接替换，避免出现断层
        if (idx < 0) {
          rows.value = withKeys(data.entries, Date.now())
          nextBefore.value = data.next_before ?? null
          freshKeys.value = new Set()
        } else if (idx > 0) {
          const added = withKeys(data.entries.slice(0, idx), Date.now())
          rows.value = [...added, ...rows.value].slice(0, FOLLOW_MAX_ROWS)
          freshKeys.value = new Set(added.map((r) => r.key))
        }
      } else {
        rows.value = withKeys(data.entries, 0)
        nextBefore.value = data.next_before ?? null
        freshKeys.value = new Set()
      }
    } catch (e) {
      if (seq === requestSeq) error.value = e
    } finally {
      if (seq === requestSeq) loading.value = false
    }
  }

  async function loadMore() {
    if (nextBefore.value == null || loadingMore.value || !file.value) return
    loadingMore.value = true
    const seq = requestSeq
    try {
      const { data } = await listKernelLogEntries({
        file: file.value,
        levels: levels.value,
        q: keyword.value,
        limit: PAGE_SIZE,
        before: nextBefore.value,
      })
      if (seq !== requestSeq) return
      rows.value = [...rows.value, ...withKeys(data.entries, rows.value.length)]
      nextBefore.value = data.next_before ?? null
    } catch (e) {
      error.value = e
    } finally {
      loadingMore.value = false
    }
  }

  async function refresh() {
    try {
      await loadOverview()
      error.value = null
    } catch (e) {
      error.value = e
      return
    }
    await Promise.all([reload(), loadStats()])
  }

  /** 跟随：只在文件变大时拉取，空转时只有一次轻量概览请求 */
  async function followTick() {
    if (!isViewingCurrent.value || loading.value) return
    try {
      const { data } = await getKernelLogOverview()
      overview.value = data
      const cur = data.files.find((f) => f.name === file.value)
      if (cur && cur.size !== lastFileSize) {
        await Promise.all([reload({ merge: true }), loadStats()])
      }
    } catch {
      // 跟随期间的瞬时失败静默，下个周期重试
    }
  }

  function toggleLevel(level: LogLevel) {
    const set = new Set(levels.value)
    if (set.has(level)) set.delete(level)
    else set.add(level)
    // 保持五级固定顺序，便于 URL / 缓存 key 稳定
    levels.value = LOG_LEVELS.filter((l) => set.has(l))
  }

  function clearLevels() {
    levels.value = []
  }

  async function download(name = file.value) {
    if (!name) return
    const { data } = await downloadKernelLogFile(name)
    const url = URL.createObjectURL(data)
    const a = document.createElement('a')
    a.href = url
    a.download = name
    a.click()
    URL.revokeObjectURL(url)
  }

  async function removeFile(name: string) {
    await deleteKernelLogFile(name)
    if (file.value === name) file.value = undefined
    await refresh()
  }

  async function changeLevel(level: LogLevel) {
    const { data } = await setKernelLogLevel(level)
    if (overview.value) overview.value = { ...overview.value, level: data.level }
    return data.level
  }

  function syncFollowTimer(on: boolean) {
    if (followTimer) clearInterval(followTimer)
    followTimer = on ? setInterval(followTick, FOLLOW_INTERVAL_MS) : null
    if (on) void followTick()
  }
  watch(following, syncFollowTimer)
  // 路由 keepAlive：离开页面暂停轮询，回来时恢复（不重置用户的跟随开关）
  onDeactivated(() => {
    if (followTimer) clearInterval(followTimer)
    followTimer = null
  })
  onActivated(() => {
    if (following.value && !followTimer) syncFollowTimer(true)
  })

  watch(file, (cur, prev) => {
    if (cur === prev || prev === undefined) return
    void Promise.all([reload(), loadStats()])
  })
  watch(levels, () => void reload())
  watch(keyword, () => {
    if (searchTimer) clearTimeout(searchTimer)
    searchTimer = setTimeout(() => void reload(), SEARCH_DEBOUNCE_MS)
  })

  onBeforeUnmount(() => {
    if (followTimer) clearInterval(followTimer)
    if (searchTimer) clearTimeout(searchTimer)
  })

  return {
    overview,
    enabled,
    files,
    file,
    currentFile,
    isViewingCurrent,
    levels,
    keyword,
    rows,
    counts,
    countsTruncated,
    loading,
    loadingMore,
    hasMore,
    error,
    following,
    freshKeys,
    refresh,
    reload,
    loadMore,
    toggleLevel,
    clearLevels,
    download,
    removeFile,
    changeLevel,
  }
}
