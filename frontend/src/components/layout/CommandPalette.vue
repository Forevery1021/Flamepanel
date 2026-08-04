<template>
  <Dialog
    v-model:visible="model"
    :modal="true"
    :closable="false"
    :dismissable-mask="true"
    :draggable="false"
    class="command-palette"
  >
    <div class="palette">
      <div class="palette-search">
        <i class="oi oi-search" />
        <input
          ref="inputRef"
          v-model="query"
          class="palette-input"
          placeholder="搜索菜单或应用…"
          @keydown.down.prevent="move(1)"
          @keydown.up.prevent="move(-1)"
          @keydown.enter.prevent="go"
        />
        <kbd>ESC</kbd>
      </div>
      <div class="palette-results">
        <div v-if="filtered.length" class="palette-group">
          <div
            v-for="(r, i) in filtered"
            :key="r.id"
            class="palette-item"
            :class="{ 'is-selected': i === cursor }"
            @mouseenter="cursor = i"
            @click="run(r)"
          >
            <i class="oi palette-item__icon" :class="r.icon" />
            <span class="palette-item__label">{{ r.label }}</span>
            <span v-if="r.hint" class="palette-item__hint">{{ r.hint }}</span>
          </div>
        </div>
        <div v-else class="palette-empty">{{ t('common.noData') }}</div>
      </div>
    </div>
  </Dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import Dialog from 'openvue/dialog'
import { useThemeStore } from '@/stores/theme'
import { menuRoutes } from '@/router'

const model = defineModel<boolean>({ required: true })

const { t } = useI18n()
const router = useRouter()
const themeStore = useThemeStore()

interface PaletteItem {
  id: string
  label: string
  icon: string
  hint?: string
  run: () => void
}

const query = ref('')
const cursor = ref(0)
const inputRef = ref<HTMLInputElement>()

/** 菜单项来自路由表（meta.title + meta.icon），与侧边栏同源 */
const staticItems: PaletteItem[] = [
  ...menuRoutes
    .filter((r) => r.meta?.group)
    .map((r) => ({
      id: String(r.name),
      label: t(r.meta?.title ?? ''),
      icon: r.meta?.icon ?? 'oi-circle',
      run: () => router.push(r.path),
    })),
  {
    id: 'toggle-dark',
    label: t('topbar.toggleDark'),
    icon: themeStore.mode === 'dark' ? 'oi-sun' : 'oi-moon',
    run: () => themeStore.setMode(themeStore.mode === 'dark' ? 'light' : 'dark'),
  },
]

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase()
  if (!q) return staticItems
  return staticItems.filter((i) => i.label.toLowerCase().includes(q))
})

function move(dir: number) {
  cursor.value = (cursor.value + dir + filtered.value.length) % filtered.value.length
}

function run(item: PaletteItem) {
  item.run()
  model.value = false
}

function go() {
  if (filtered.value[cursor.value]) run(filtered.value[cursor.value])
}

watch(model, async (open) => {
  if (open) {
    query.value = ''
    cursor.value = 0
    await nextTick()
    inputRef.value?.focus()
  }
})
</script>

<style scoped>
.command-palette {
  border-radius: var(--fp-radius-lg);
  overflow: hidden;
  box-shadow: 0 24px 64px -12px rgb(0 0 0 / 0.35);
}
.palette {
  min-width: 520px;
  max-width: 90vw;
}
.palette-search {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 16px;
  border-bottom: 1px solid var(--fp-border);
  color: var(--fp-text-muted);
}
.palette-input {
  flex: 1;
  border: none;
  outline: none;
  background: transparent;
  font-size: 15px;
  color: var(--fp-text-primary);
  font-family: var(--fp-font-sans);
}
.palette-search kbd {
  font-family: var(--fp-font-mono);
  font-size: 10px;
  color: var(--fp-text-muted);
  border: 1px solid var(--fp-border-strong);
  border-radius: 4px;
  padding: 2px 6px;
}
.palette-results {
  max-height: 320px;
  overflow-y: auto;
  padding: 8px;
}
.palette-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 12px;
  border-radius: var(--fp-radius-sm);
  cursor: pointer;
  font-size: 14px;
  color: var(--fp-text-primary);
}
.palette-item.is-selected {
  background: var(--fp-bg-hover);
  color: var(--fp-text-primary);
}
.palette-item__icon {
  width: 18px;
  text-align: center;
  color: var(--fp-text-secondary);
  font-size: 14px;
}
.palette-item.is-selected .palette-item__icon {
  color: var(--fp-brand);
}
.palette-item__label {
  flex: 1;
}
.palette-item__hint {
  font-size: 11px;
  color: var(--fp-text-muted);
}
.palette-empty {
  padding: 24px;
  text-align: center;
  color: var(--fp-text-muted);
  font-size: 13px;
}
@media (max-width: 600px) {
  .palette {
    min-width: 0;
    width: 88vw;
  }
}
</style>
