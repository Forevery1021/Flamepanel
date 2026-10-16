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
        <i class="oi oi-search" aria-hidden="true" />
        <input
          ref="inputRef"
          v-model="query"
          class="palette-input"
          :placeholder="t('topbar.searchPlaceholder')"
          role="combobox"
          aria-label="Command"
          aria-autocomplete="list"
          :aria-expanded="model"
          aria-controls="palette-listbox"
          :aria-activedescendant="filtered.length ? `palette-option-${cursor}` : undefined"
          @keydown.down.prevent="move(1)"
          @keydown.up.prevent="move(-1)"
          @keydown.home.prevent="cursor = 0"
          @keydown.end.prevent="cursor = filtered.length - 1"
          @keydown.enter.prevent="go"
          @keydown.esc.prevent="model = false"
        />
        <kbd aria-hidden="true">ESC</kbd>
      </div>
      <div id="palette-listbox" class="palette-results" role="listbox" aria-label="Commands">
        <template v-if="filtered.length">
          <div
            v-for="g in groups"
            :key="g.key"
            class="palette-group"
            role="group"
            :aria-label="g.label || undefined"
          >
            <div v-if="g.label" class="palette-group__label" aria-hidden="true">{{ g.label }}</div>
            <div
              v-for="(r, j) in g.items"
              :id="`palette-option-${g.offset + j}`"
              :key="r.id"
              class="palette-item"
              role="option"
              :aria-selected="g.offset + j === cursor"
              :class="{ 'is-selected': g.offset + j === cursor }"
              @mouseenter="cursor = g.offset + j"
              @click="run(r)"
            >
              <i class="oi palette-item__icon" :class="r.icon" aria-hidden="true" />
              <span class="palette-item__label">{{ r.label }}</span>
              <span v-if="r.hint" class="palette-item__hint">{{ r.hint }}</span>
            </div>
          </div>
        </template>
        <div v-else class="palette-empty" role="status">{{ t('common.noData') }}</div>
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
import { useAuthStore } from '@/stores/auth'
import { readRecentRoutes } from '@/utils/recentRoutes'
import {
  buildNavigationCommands,
  buildActionCommands,
  buildRecentCommands,
  type CommandItem,
} from '@/config/commands'

const model = defineModel<boolean>({ required: true })

const { t } = useI18n()
const router = useRouter()
const themeStore = useThemeStore()
const auth = useAuthStore()

const query = ref('')
const cursor = ref(0)
const inputRef = ref<HTMLInputElement>()
/** 最近访问的路由名：每次打开面板时从存储重读（导航发生在面板关闭期间） */
const recentNames = ref<string[]>([])

/** Modernization M1：命令统一来自 config/commands.ts（导航命令 + 动作命令集中注册） */
const navigation = computed<CommandItem[]>(() => buildNavigationCommands(t, router))
const staticItems = computed<CommandItem[]>(() => [
  ...navigation.value,
  ...buildActionCommands(t, themeStore),
])
const recentItems = computed(() => buildRecentCommands(navigation.value, recentNames.value, t))

interface PaletteGroup {
  key: string
  label: string
  items: CommandItem[]
  /** 本组首项在扁平列表中的下标（键盘游标与 aria-activedescendant 共用扁平下标） */
  offset: number
}

/**
 * S-15：空查询时分「最近访问 / 全部命令」两组；有查询时只按关键字过滤全部命令
 * （最近访问是全部命令的子集，查询态再列一遍只会产生重复项）。
 */
const groups = computed<PaletteGroup[]>(() => {
  const q = query.value.trim().toLowerCase()
  if (q) {
    const items = staticItems.value.filter((i) => {
      if (i.label.toLowerCase().includes(q)) return true
      return (i.keywords ?? []).some((k) => k.toLowerCase().includes(q))
    })
    return items.length ? [{ key: 'results', label: '', items, offset: 0 }] : []
  }
  if (!recentItems.value.length) {
    return [{ key: 'all', label: '', items: staticItems.value, offset: 0 }]
  }
  return [
    { key: 'recent', label: t('palette.recent'), items: recentItems.value, offset: 0 },
    {
      key: 'all',
      label: t('palette.allCommands'),
      items: staticItems.value,
      offset: recentItems.value.length,
    },
  ]
})

const filtered = computed(() => groups.value.flatMap((g) => g.items))

function move(dir: number) {
  cursor.value = (cursor.value + dir + filtered.value.length) % filtered.value.length
}

function run(item: CommandItem) {
  item.run()
  model.value = false
}

function go() {
  if (filtered.value[cursor.value]) run(filtered.value[cursor.value])
}

watch(query, () => {
  cursor.value = 0
})

watch(model, async (open) => {
  if (open) {
    query.value = ''
    cursor.value = 0
    recentNames.value = readRecentRoutes(auth.username)
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
.palette-group + .palette-group {
  margin-top: 6px;
  padding-top: 6px;
  border-top: 1px solid var(--fp-border);
}
.palette-group__label {
  padding: 4px 12px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.04em;
  color: var(--fp-text-muted);
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
