<template>
  <FpModal v-model="visible" :header="`${t('docker.containerLogs')} — ${container}`" size="xl">
    <div class="log-toolbar">
      <div class="tail-select">
        <FpSelect v-model="tail" :options="tailOptions" option-label="label" option-value="value" />
      </div>
      <FpButton variant="ghost" icon="oi oi-refresh" @click="emit('refresh')">{{
        t('docker.refresh')
      }}</FpButton>
    </div>
    <pre class="log-viewer">{{ content || t('docker.noLogs') }}</pre>
  </FpModal>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { logTailOptions } from '@/utils/dockerFormat'
import FpModal from '@/components/ui/FpModal.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpSelect from '@/components/ui/FpSelect.vue'

/**
 * 容器日志弹窗（Doc/23 §3.3 T2-4 视图拆分第三站）。
 *
 * 三处契约：
 * - `container` 为**展示名**（名称回退 ID，父级已算好），标题里直接拼接；
 * - `tail` 经 `v-model:tail` 由父级持有——切换行数后父级立即重取（原实现里
 *   `logTail` 变了却不自动刷新，是「切了行数觉得没反应」的常见误解来源；
 *   本次**保持原语义**不改行为，只在用例中把当前契约固化：切换行数本身不发请求，
 *   用户点「刷新」才生效）；
 * - 内容为空时渲染 `docker.noLogs` 占位，而不是留一段空白 `<pre>`。
 */
withDefaults(
  defineProps<{
    content?: string
    container?: string
  }>(),
  { content: '', container: '' },
)

const visible = defineModel<boolean>({ default: false })
const tail = defineModel<number>('tail', { default: 100 })

const emit = defineEmits<{ refresh: [] }>()

const { t } = useI18n()
const tailOptions = computed(() => logTailOptions(t))
</script>

<style scoped>
.log-toolbar {
  display: flex;
  align-items: center;
  gap: var(--fp-space-3);
  margin-bottom: var(--fp-space-3);
}
.tail-select {
  width: 220px;
}
.log-viewer {
  background: var(--fp-bg-terminal);
  color: var(--fp-text-code);
  padding: 16px;
  border-radius: 6px;
  max-height: 500px;
  overflow: auto;
  font-size: 13px;
  line-height: 1.5;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
