import { computed, ref, watch } from 'vue'
import { listFiles } from '@/api/files'
import { queryKeys } from '@/api/queryKeys'
import { useApiQuery, useQueryCacheClient } from '@/composables/useApiQuery'
import type { FileInfo } from '@/types'
import { buildCrumbs, normalizeDir, pruneSelection, toggleSelection } from '@/utils/fileBrowser'

/**
 * 文件管理数据层：当前目录 + 列表查询（按路径缓存）+ 多选。
 *
 * - 切换目录清空选择；列表刷新后丢弃已不存在的选中项；
 * - 写操作完成后调用 `invalidate()`，让所有目录缓存失效（复制 / 移动可能影响别的目录）。
 */
export function useFileBrowser(initialPath = '/') {
  const queryClient = useQueryCacheClient()
  const currentPath = ref(normalizeDir(initialPath))

  const query = useApiQuery<FileInfo[]>(
    () => queryKeys.files.list(currentPath.value),
    () => listFiles(currentPath.value),
  )
  const entries = computed<FileInfo[]>(() => query.data.value ?? [])
  const crumbs = computed(() => buildCrumbs(currentPath.value))

  const selected = ref<string[]>([])
  const selectedRows = computed(() => entries.value.filter((e) => selected.value.includes(e.path)))
  const allSelected = computed(
    () => entries.value.length > 0 && selected.value.length === entries.value.length,
  )

  watch(currentPath, () => {
    selected.value = []
  })
  watch(entries, (list) => {
    selected.value = pruneSelection(selected.value, list)
  })

  function goTo(path: string) {
    currentPath.value = normalizeDir(path)
  }

  function toggle(path: string, on: boolean) {
    selected.value = toggleSelection(selected.value, path, on)
  }

  function toggleAll(on: boolean) {
    selected.value = on ? entries.value.map((e) => e.path) : []
  }

  function clearSelection() {
    selected.value = []
  }

  function invalidate() {
    void queryClient.invalidateQueries({ queryKey: ['files', 'list'] })
  }

  async function refresh() {
    await query.refresh()
  }

  return {
    currentPath,
    crumbs,
    entries,
    loading: query.loading,
    error: query.error,
    refresh,
    invalidate,
    goTo,
    selected,
    selectedRows,
    allSelected,
    toggle,
    toggleAll,
    clearSelection,
  }
}
