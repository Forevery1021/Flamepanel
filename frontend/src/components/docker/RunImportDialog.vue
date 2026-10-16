<template>
  <FpModal v-model="visible" :header="t('docker.importRun')" size="md">
    <form class="modal-form" @submit.prevent="submit">
      <p class="hint">{{ t('docker.importRunHint') }}</p>
      <FpTextarea
        v-model="command"
        :label="t('docker.runCommand')"
        :placeholder="t('docker.runCommandPlaceholder')"
        :error="error"
        :rows="8"
        class="command-textarea w-full"
      />
    </form>
    <template #footer>
      <FpButton variant="ghost" @click="close">{{ t('common.cancel') }}</FpButton>
      <FpButton variant="primary" :loading="submitting" @click="submit">{{
        t('docker.convert')
      }}</FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import FpModal from '@/components/ui/FpModal.vue'
import FpTextarea from '@/components/ui/FpTextarea.vue'
import FpButton from '@/components/ui/FpButton.vue'

/**
 * `docker run` 粘贴导入弹窗（Doc/30 M-D）。
 *
 * 只负责收集命令并上抛；转换请求、回填 Compose 表单与「成功才关」由父级编排
 * （失败保持打开，让用户就地修改命令）。转换结果不会自动部署。
 */
withDefaults(defineProps<{ submitting?: boolean }>(), { submitting: false })

const emit = defineEmits<{ submit: [command: string] }>()

const { t } = useI18n()

const visible = ref(false)
const command = ref('')
const error = ref('')

function open() {
  error.value = ''
  visible.value = true
}

function close() {
  visible.value = false
}

function submit() {
  const value = command.value.trim()
  if (!value) {
    error.value = t('common.required')
    return
  }
  error.value = ''
  emit('submit', value)
}

defineExpose({ open, close })
</script>

<style scoped>
.hint {
  margin: 0 0 var(--fp-space-2);
  color: var(--fp-text-secondary);
  font-size: var(--fp-font-size-sm);
}
.command-textarea {
  font-family: var(--fp-font-mono);
}
</style>
