<template>
  <nav class="tool-grid" :aria-label="t('toolbox.tools.title')">
    <RouterLink v-for="tool in tools" :key="tool.name" :to="tool.path" class="tool-card">
      <i :class="['oi', tool.icon, 'tool-icon']" aria-hidden="true" />
      <span class="tool-title">{{ t(tool.title) }}</span>
      <span class="tool-desc text-muted">{{ t(tool.desc) }}</span>
    </RouterLink>
  </nav>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink } from 'vue-router'
import { menuRoutes } from '@/router'
import { hasPermission } from '@/directives/permission'
import { useAuthStore } from '@/stores/auth'
import { TOOL_ROUTE_NAMES } from '@/utils/hostInfo'

const { t } = useI18n()
const auth = useAuthStore()

/** 只展示当前角色有权限进入的工具；顺序以 `TOOL_ROUTE_NAMES` 为准 */
const tools = computed(() =>
  TOOL_ROUTE_NAMES.flatMap((name) => {
    const route = menuRoutes.find((r) => r.name === name)
    if (!route) return []
    const perm = route.meta.perm
    if (perm && !hasPermission(auth.role, perm)) return []
    return [
      {
        name,
        path: route.path,
        icon: route.meta.icon,
        title: route.meta.title,
        desc: `toolbox.tools.desc.${name}`,
      },
    ]
  }),
)
</script>

<style scoped>
.tool-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: var(--fp-space-3);
}
.tool-card {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  grid-template-rows: auto auto;
  column-gap: var(--fp-space-3);
  row-gap: 2px;
  padding: var(--fp-space-3) var(--fp-space-4);
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-surface);
  color: inherit;
  text-decoration: none;
  transition:
    border-color var(--fp-transition-fast),
    background var(--fp-transition-fast);
}
.tool-card:hover,
.tool-card:focus-visible {
  border-color: var(--fp-brand);
  background: var(--fp-bg-hover);
}
.tool-card:focus-visible {
  outline: none;
  box-shadow: var(--fp-focus-ring);
}
.tool-icon {
  grid-row: 1 / span 2;
  align-self: center;
  font-size: 20px;
  color: var(--fp-brand);
}
.tool-title {
  font-weight: 600;
  font-size: var(--fp-font-sm);
}
.tool-desc {
  font-size: var(--fp-font-xs);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
