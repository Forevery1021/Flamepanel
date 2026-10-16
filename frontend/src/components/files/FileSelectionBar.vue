<template>
  <div class="selection-bar" role="region" :aria-label="t('file.selectionActions')">
    <span aria-live="polite">{{ t('file.selected', { n: count }) }}</span>
    <FpButton
      v-permission="{ perm: 'file:write', mode: 'view' }"
      variant="ghost"
      icon="oi oi-box"
      @click="emit('compress')"
    >
      {{ t('file.compress') }}
    </FpButton>
    <FpButton
      v-permission="{ perm: 'file:write', mode: 'view' }"
      variant="ghost"
      icon="oi oi-copy"
      @click="emit('copy')"
    >
      {{ t('file.copyTo') }}
    </FpButton>
    <FpButton
      v-permission="{ perm: 'file:write', mode: 'view' }"
      variant="ghost"
      icon="oi oi-arrow-right"
      @click="emit('move')"
    >
      {{ t('file.moveTo') }}
    </FpButton>
    <FpButton
      v-permission="{ perm: 'file:write', mode: 'view' }"
      variant="danger"
      icon="oi oi-trash"
      @click="emit('delete')"
    >
      {{ t('file.delete') }}
    </FpButton>
    <FpButton variant="link" @click="emit('clear')">{{ t('file.clearSelection') }}</FpButton>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import FpButton from '@/components/ui/FpButton.vue'

defineProps<{ count: number }>()
const emit = defineEmits<{
  compress: []
  copy: []
  move: []
  delete: []
  clear: []
}>()

const { t } = useI18n()
</script>

<style scoped>
.selection-bar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--fp-space-2);
  margin-bottom: var(--fp-space-3);
  padding: var(--fp-space-2) var(--fp-space-3);
  border-radius: var(--fp-radius-md);
  background: var(--fp-brand-soft);
}
.selection-bar > span {
  margin-right: auto;
}
</style>
