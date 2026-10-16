import { ref } from 'vue'
import { listMemos, updateMemo } from '@/api/memos'
import type { Memo } from '@/types'

/** 仪表盘「今日待办」的展示上限（原实现直接写在视图里的 `slice(0, 6)`） */
export const TODO_LIMIT = 6

/**
 * 仪表盘今日待办数据层（Doc/23 §3.3 T2-4 视图拆分第五站）。
 *
 * 本族在原 851 行视图里只有两个函数，但三处语义值得固化：
 *
 * 1. **只取前 6 条**：仪表盘是「扫一眼」的场景，待办面板与备忘录页共用同一接口，
 *    全量返回会把面板撑爆。截断发生在数据层而不是模板 `v-for` 里——模板截断会让
 *    「一共加载了多少条」这件事在别处被重新解释一遍。
 * 2. **勾选先本地置位、再重拉列表**：`updateMemo` 成功后先改本地副本（用户立刻看到
 *    删除线），随后重拉——待办接口带 `done=false` 过滤，勾选后该项本就不该再出现。
 *    只做本地改动会让已完成条目继续留在「今日待办」里，与刷新后的结果不一致；
 *    只重拉则要等一次往返才有反馈。原实现即两件事都做，本次固化并有用例锁死。
 * 3. **失败静默回落空列表**（原实现 `catch { [] }`）：仪表盘是只读概览，弹错误
 *    提示会打断用户在其他面板上的操作；失败后展示空态，下一次进入页面自然收敛。
 */
export function useDashboardTodos() {
  const todos = ref<Memo[]>([])
  const loading = ref(false)

  async function fetch() {
    loading.value = true
    try {
      const res = await listMemos('todo', false)
      todos.value = res.data.slice(0, TODO_LIMIT)
    } catch {
      todos.value = []
    } finally {
      loading.value = false
    }
  }

  /** 勾选/取消勾选（见文件头语义 2）；失败静默 */
  async function toggle(todo: Memo, done: boolean) {
    try {
      await updateMemo(todo.id, { done })
      todo.done = done
      await fetch()
    } catch {
      // 仪表盘为只读概览：失败不打断用户，下一次拉取自然收敛
    }
  }

  return { todos, loading, fetch, toggle }
}
