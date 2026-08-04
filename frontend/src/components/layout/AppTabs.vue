<template>
  <div class="app-tabs" :class="{ 'is-mobile': isMobile }">
    <div class="tabs-track">
      <button
        v-for="tab in tabsStore.tabs"
        :key="tab.path"
        class="tab-item"
        :class="{ 'is-active': tab.path === route.path }"
        @click="goto(tab.path)"
        @middleclick.prevent="tabsStore.close(tab.path)"
        @contextmenu.prevent.stop="openContext(tab.path, $event)"
      >
        <span class="tab-dot" aria-hidden="true" />
        <span class="tab-label">{{ tab.title }}</span>
        <span
          v-if="!tab.pinned"
          class="tab-close"
          role="button"
          tabindex="-1"
          @click.stop="tabsStore.close(tab.path)"
        >
          <i class="oi oi-times" />
        </span>
      </button>
    </div>

    <Menu ref="ctxMenuRef" :model="ctxItems" popup />
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import Menu from 'openvue/menu'
import type { MenuItem } from 'openvue/menuitem'
import { useTabsStore } from '@/stores/tabs'

defineProps<{ isMobile?: boolean }>()

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const tabsStore = useTabsStore()

const ctxMenuRef = ref<InstanceType<typeof Menu>>()
const ctxPath = ref('')

function goto(path: string) {
  if (route.path !== path) router.push(path)
}

function openContext(path: string, event: MouseEvent) {
  ctxPath.value = path
  ctxItems.value = [
    {
      label: t('tabs.refresh'),
      icon: 'oi oi-refresh',
      command: () => {
        if (route.path === path) router.replace({ path, query: { t: Date.now().toString() } })
        else router.push(path)
      },
    },
    { separator: true },
    {
      label: t('tabs.close'),
      icon: 'oi oi-times',
      command: () => tabsStore.close(ctxPath.value),
    },
    {
      label: t('tabs.closeOthers'),
      icon: 'oi oi-window-minimize',
      command: () => tabsStore.closeOthers(ctxPath.value),
    },
    {
      label: t('tabs.closeAll'),
      icon: 'oi oi-trash',
      command: () => tabsStore.closeAll(),
    },
  ]
  ctxMenuRef.value?.toggle(event)
}

const ctxItems = ref<MenuItem[]>([])
</script>

<style scoped>
.app-tabs {
  display: flex;
  align-items: center;
  height: 38px;
  padding: 0 var(--fp-space-3);
  border-bottom: 1px solid var(--fp-border);
  background: var(--fp-bg-elevated);
  flex-shrink: 0;
  overflow-x: auto;
  scrollbar-width: none;
}
.app-tabs::-webkit-scrollbar {
  display: none;
}
.tabs-track {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: max-content;
}
.tab-item {
  display: flex;
  align-items: center;
  gap: 6px;
  height: 28px;
  padding: 0 8px 0 10px;
  border: 1px solid transparent;
  border-radius: var(--fp-radius-sm);
  background: transparent;
  color: var(--fp-text-secondary);
  font-size: 12.5px;
  font-family: var(--fp-font-sans);
  cursor: pointer;
  user-select: none;
  transition:
    background-color var(--fp-transition-fast),
    color var(--fp-transition-fast),
    border-color var(--fp-transition-fast);
}
.tab-item:hover {
  background: var(--fp-bg-hover);
  color: var(--fp-text-primary);
}
.tab-item.is-active {
  background: var(--fp-brand-soft);
  border-color: color-mix(in srgb, var(--fp-brand) 30%, transparent);
  color: var(--fp-brand);
  font-weight: 600;
}
.tab-dot {
  width: 5px;
  height: 5px;
  border-radius: 999px;
  background: var(--fp-text-muted);
  flex-shrink: 0;
  transition: background-color var(--fp-transition-fast);
}
.tab-item.is-active .tab-dot {
  background: var(--fp-brand);
}
.tab-label {
  max-width: 140px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tab-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  border-radius: 4px;
  font-size: 9px;
  opacity: 0;
  transition: opacity var(--fp-transition-fast);
}
.tab-item:hover .tab-close,
.tab-item.is-active .tab-close {
  opacity: 0.7;
}
.tab-close:hover {
  opacity: 1;
  background: var(--fp-bg-hover);
}

.is-mobile {
  padding: 0 var(--fp-space-2);
}
</style>
