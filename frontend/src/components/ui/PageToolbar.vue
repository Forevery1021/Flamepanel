<template>
  <div class="page-toolbar">
    <div class="toolbar-left">
      <!-- 搜索框（通过 v-model 传入 search） -->
      <div v-if="searchable" class="toolbar-search">
        <i class="oi oi-search toolbar-search__icon" aria-hidden="true" />
        <FpInput
          ref="searchRef"
          v-model="search"
          :placeholder="searchPlaceholder || t('common.searchPlaceholder')"
          class="toolbar-search__input"
        />
        <kbd v-if="!search" class="toolbar-search__kbd" aria-hidden="true">/</kbd>
      </div>
      <!-- 左侧扩展（筛选器等） -->
      <slot name="left" />
    </div>
    <div class="toolbar-right">
      <slot name="actions" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import FpInput from './FpInput.vue'

const props = withDefaults(
  defineProps<{
    /** 是否显示搜索框 */
    searchable?: boolean
    /** 搜索值（v-model） */
    modelValue?: string
    /** 搜索框占位文案 */
    searchPlaceholder?: string
  }>(),
  {
    searchable: true,
    modelValue: '',
    searchPlaceholder: '',
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const { t } = useI18n()

const search = computed({
  get: () => props.modelValue,
  set: (v: string) => emit('update:modelValue', v),
})

/** `/` 快捷聚焦搜索框（焦点不在输入控件内时） */
const searchRef = ref<{ $el?: HTMLElement } | null>(null)
function onKey(e: KeyboardEvent) {
  if (e.key !== '/' || e.metaKey || e.ctrlKey || e.altKey) return
  const tag = (e.target as HTMLElement | null)?.tagName
  if (tag === 'INPUT' || tag === 'TEXTAREA' || (e.target as HTMLElement | null)?.isContentEditable)
    return
  const el = searchRef.value?.$el?.querySelector?.('input') ?? searchRef.value?.$el
  if (el instanceof HTMLElement) {
    e.preventDefault()
    el.focus()
  }
}
onMounted(() => props.searchable && window.addEventListener('keydown', onKey))
onUnmounted(() => window.removeEventListener('keydown', onKey))
</script>

<style scoped>
.page-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--fp-space-3);
  margin-bottom: var(--fp-space-4);
  flex-wrap: wrap;
}
.toolbar-left {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  flex-wrap: wrap;
}
/* 工具条内的 Fp 表单控件按内容取宽，不撑满换行（.fp-field 默认 width:100%） */
.toolbar-left :deep(.fp-field),
.toolbar-right :deep(.fp-field) {
  width: auto;
}
.toolbar-left :deep(.fp-field > .p-select),
.toolbar-left :deep(.fp-field > .p-inputtext) {
  min-width: 160px;
}
.toolbar-right {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  flex-wrap: wrap;
}
.toolbar-search {
  position: relative;
  width: 260px;
  display: flex;
  align-items: center;
}
.toolbar-search__input {
  width: 100%;
}
.toolbar-search__input :deep(input) {
  padding-left: 32px;
  padding-right: 30px;
}
.toolbar-search__icon {
  position: absolute;
  left: 11px;
  z-index: 1;
  font-size: 13px;
  color: var(--fp-text-muted);
  pointer-events: none;
}
.toolbar-search__kbd {
  position: absolute;
  right: 8px;
  z-index: 1;
  padding: 0 5px;
  border: 1px solid var(--fp-border);
  border-radius: 4px;
  font: 11px/16px var(--fp-font-mono);
  color: var(--fp-text-muted);
  background: var(--fp-bg-subtle);
  pointer-events: none;
}
/* 手机：左右两组各占一行，搜索框与筛选撑满（旧版固定 260px / 160px，右侧留大片空白） */
@media (max-width: 640px) {
  .toolbar-left,
  .toolbar-right {
    width: 100%;
  }
  .toolbar-search {
    width: 100%;
  }
  .toolbar-left :deep(.fp-field) {
    flex: 1 1 140px;
  }
  .toolbar-left :deep(.fp-field > .p-select),
  .toolbar-left :deep(.fp-field > .p-inputtext) {
    min-width: 0;
  }
}
</style>
