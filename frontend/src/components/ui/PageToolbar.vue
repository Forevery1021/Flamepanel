<template>
  <div class="page-toolbar">
    <div class="toolbar-left">
      <!-- 搜索框（通过 v-model 传入 search） -->
      <FpInput
        v-if="searchable"
        v-model="search"
        :placeholder="searchPlaceholder || t('common.searchPlaceholder')"
        class="toolbar-search"
      />
      <!-- 左侧扩展（筛选器等） -->
      <slot name="left" />
    </div>
    <div class="toolbar-right">
      <slot name="actions" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
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
.toolbar-right {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  flex-wrap: wrap;
}
.toolbar-search {
  width: 240px;
}
</style>
