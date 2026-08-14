<template>
  <div class="layout-content">
    <div class="lc-header">
      <div class="lc-title">
        <h2 class="lc-title__text">{{ title }}</h2>
        <div v-if="$slots.prompt" class="lc-prompt">
          <slot name="prompt" />
        </div>
      </div>
      <div class="lc-toolbar">
        <div v-if="$slots.toolbar" class="lc-toolbar__left">
          <slot name="toolbar" />
        </div>
        <div class="lc-toolbar__right">
          <slot name="actions" />
          <FpButton
            v-if="reload"
            variant="ghost"
            icon="oi oi-refresh"
            :loading="reloading"
            :aria-label="t('common.refresh')"
            :title="t('common.refresh')"
            @click="doReload"
          />
        </div>
      </div>
    </div>
    <div class="lc-body">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import FpButton from './FpButton.vue'

const props = withDefaults(
  defineProps<{
    title?: string
    /** 是否显示刷新按钮 */
    reload?: boolean
  }>(),
  { title: '', reload: false },
)

const emit = defineEmits<{ reload: [] }>()

const { t } = useI18n()
const reloading = ref(false)

async function doReload() {
  if (props.reload) {
    reloading.value = true
    emit('reload')
    await new Promise((r) => setTimeout(r, 500))
    reloading.value = false
  }
}
</script>

<style scoped>
.layout-content {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
}
.lc-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--fp-space-4);
  flex-wrap: wrap;
}
.lc-title__text {
  font-size: 18px;
  font-weight: 700;
  letter-spacing: -0.01em;
  color: var(--fp-text-primary);
}
.lc-prompt {
  margin-top: var(--fp-space-2);
  max-width: 720px;
}
.lc-toolbar {
  display: flex;
  align-items: center;
  gap: var(--fp-space-3);
  flex-wrap: wrap;
}
.lc-toolbar__left,
.lc-toolbar__right {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  flex-wrap: wrap;
}
.lc-body {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
  min-width: 0;
}
</style>
