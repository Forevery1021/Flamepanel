<template>
  <div class="panel">
    <div class="panel-header">
      <span class="panel-title">{{ t('dashboard.commonApps') }}</span>
    </div>
    <div v-if="apps.length" class="common-apps">
      <button
        v-for="a in apps"
        :key="a.id"
        class="common-app"
        :disabled="disabled"
        @click="emit('open', a)"
      >
        <span class="common-app-icon"><i class="oi oi-box" /></span>
        <span class="common-app-name">{{ a.name }}</span>
        <span class="common-app-count mono">{{ a.launch_count ?? 0 }}</span>
      </button>
    </div>
    <div v-else class="panel-empty"><i class="oi oi-box panel-empty__icon" aria-hidden="true" />{{ t('dashboard.noCommonApps') }}</div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { InstalledApp } from '@/api/appStore'

/**
 * 常用应用面板（Doc/23 §3.3 T2-4 视图拆分第五站）。
 *
 * **排序与截断不在本组件**：`launch_count` 降序取前 6 由
 * `useDashboardApps` 负责——「常用」的定义必须在数据层有唯一落点，
 * 否则模板里的 `sort` 会变成一句无人负责的代码。
 *
 * 启动次数缺失回落 `0`（`?? 0`）：老库/未走过的应用没有该字段，
 * 渲染成空字符串会让这一列看起来像「加载失败」。
 *
 * viewer 只读：按钮置 `disabled`（**不禁用整个面板**——应用清单本身是可见信息，
 * 只是不能点开）。图标沿用统一占位 `oi-box`，真实图标属 T3-4。
 */
defineProps<{
  apps: InstalledApp[]
  /** viewer 只读：按钮置灰不可点 */
  disabled: boolean
}>()

const emit = defineEmits<{ open: [app: InstalledApp] }>()

const { t } = useI18n()
</script>

<style scoped>
.common-apps {
  display: flex;
  flex-direction: column;
}
.common-app {
  display: flex;
  align-items: center;
  gap: var(--fp-space-3);
  padding: var(--fp-space-2);
  border: none;
  background: transparent;
  border-radius: var(--fp-radius-sm);
  cursor: pointer;
  text-align: left;
  font-family: var(--fp-font-sans);
  transition: background-color var(--fp-transition-fast);
}
.common-app:hover {
  background: var(--fp-bg-hover);
}
.common-app-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: var(--fp-radius-sm);
  background: var(--fp-brand-soft);
  color: var(--fp-brand);
  font-size: 15px;
}
.common-app-name {
  flex: 1;
  font-size: 13.5px;
  color: var(--fp-text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.common-app-count {
  font-size: 12px;
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
