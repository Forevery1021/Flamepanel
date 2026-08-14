import { ref, computed } from 'vue'

/**
 * P4 · useCrudPage — 统一 CRUD 列表页的通用状态与分页逻辑。
 *
 * 收敛各列表页重复的 `currentPage / pageSize / total / goPage` 与搜索、分页
 * 组件绑定参数，降低样板代码。
 *
 * 用法：
 *   const crud = useCrudPage()
 *   // 模板
 *   <FpPagination v-if="crud.total > crud.pageSize" :first="crud.first"
 *     :rows="crud.pageSize" :total="crud.total" @update:first="crud.onFirst" />
 */
export interface UseCrudPageOptions {
  /** 初始页码（从 1 开始） */
  initialPage?: number
  /** 每页条数 */
  pageSize?: number
}

export function useCrudPage(options: UseCrudPageOptions = {}) {
  const { initialPage = 1, pageSize: initPageSize = 20 } = options

  /** 当前页（1 起） */
  const currentPage = ref(initialPage)
  /** 每页条数 */
  const pageSize = ref(initPageSize)
  /** 总条数 */
  const total = ref(0)

  /** FpPagination 的 first（偏移量，0 起） */
  const first = computed(() => (currentPage.value - 1) * pageSize.value)

  /** FpPagination update:first → 页号 */
  function onFirst(firstVal: number) {
    currentPage.value = Math.floor(firstVal / pageSize.value) + 1
  }

  /** 翻页（显式页号） */
  function goTo(page: number) {
    currentPage.value = Math.max(1, page)
  }

  /** 重置到首页（搜索/筛选变化时调用） */
  function reset() {
    currentPage.value = 1
  }

  return {
    currentPage,
    pageSize,
    total,
    first,
    onFirst,
    goTo,
    reset,
  }
}
