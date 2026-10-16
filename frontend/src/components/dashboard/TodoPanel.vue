<template>
  <div class="panel">
    <div class="panel-header">
      <span class="panel-title">{{ t('dashboard.todayTodos') }}</span>
      <FpButtonLink text size="small" @click="emit('more')">{{
        t('dashboard.more')
      }}</FpButtonLink>
    </div>
    <div v-if="loading" class="todo-skeleton">
      <FpSkeleton v-for="i in 4" :key="i" height="28px" />
    </div>
    <div v-else class="todo-list">
      <div v-for="td in todos" :key="td.id" class="todo-item" :class="{ done: td.done }">
        <FpCheckbox
          v-if="editable"
          :model-value="td.done"
          :disabled="!editable"
          @change="(v) => emit('toggle', td, Boolean(v))"
        />
        <span class="todo-content">{{ td.content }}</span>
      </div>
      <div v-if="!todos.length" class="panel-empty"><i class="oi oi-check-circle panel-empty__icon" aria-hidden="true" />{{ t('dashboard.noTodos') }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import FpButtonLink from '@/components/ui/FpButtonLink.vue'
import FpCheckbox from '@/components/ui/FpCheckbox.vue'
import FpSkeleton from '@/components/ui/FpSkeleton.vue'
import type { Memo } from '@/types'

/**
 * 今日待办面板（Doc/23 §3.3 T2-4 视图拆分第五站）。
 *
 * 三处由本组件承担的行为契约：
 *
 * 1. **只读角色不渲染勾选框**（`v-if="editable"`，不是 `:disabled`）——viewer
 *    看不到不能用的控件比看到一个灰掉的控件更清楚；`:disabled` 同时保留是防御性
 *    写法（外层若将来改成「显示但禁用」也不必改这里）。原实现即 `v-if + :disabled`
 *    双写，本次保留。
 * 2. **骨架屏与空态互斥**：`loading` 为真出 4 行骨架；否则出列表（可能为空）。
 *    先骨架后空态，避免首次加载的一瞬间闪一下「暂无待办」再出现内容。
 * 3. **`change` 载荷是布尔**（FpCheckbox 已修 C-3 缺陷）：`Boolean(v)` 是一次
 *    冗余但无害的收敛，表达「这里只要布尔」的意图；取消勾选同样会走 emit
 *    （历史缺陷下恒为 true，`MemoList.spec` / `DashboardView.spec` 有回归锁）。
 */
defineProps<{
  todos: Memo[]
  loading: boolean
  /** viewer 只读：不渲染勾选框 */
  editable: boolean
}>()

const emit = defineEmits<{
  toggle: [todo: Memo, done: boolean]
  more: []
}>()

const { t } = useI18n()
</script>

<style scoped>
.todo-skeleton {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-2);
}
.todo-list {
  display: flex;
  flex-direction: column;
}
.todo-item {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  padding: var(--fp-space-2) 0;
  border-bottom: 1px solid var(--fp-border);
}
.todo-item:last-child {
  border-bottom: none;
}
.todo-content {
  font-size: 13.5px;
  color: var(--fp-text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.todo-item.done .todo-content {
  text-decoration: line-through;
  color: var(--fp-text-muted);
}
.panel-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--fp-space-2);
  padding: var(--fp-space-6) var(--fp-space-2);
  text-align: center;
  color: var(--fp-text-muted);
  font-size: 13px;
}
.panel-empty__icon {
  font-size: 20px;
  opacity: 0.6;
}
</style>
