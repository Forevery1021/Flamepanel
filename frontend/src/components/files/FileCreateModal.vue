<template>
  <FpModal v-model="visible" :header="kind === 'dir' ? t('file.createDir') : t('file.createFile')">
    <form class="modal-form" @submit.prevent="submit">
      <FpInput
        v-model="name"
        :label="t('file.name')"
        :placeholder="kind === 'dir' ? 'dirname' : 'filename'"
      />
      <p class="hint">{{ t('file.createIn', { dir }) }}</p>
    </form>
    <template #footer>
      <FpButton variant="ghost" @click="visible = false">{{ t('file.cancel') }}</FpButton>
      <FpButton variant="primary" :disabled="!name.trim()" :loading="running" @click="submit">
        {{ t('common.confirm') }}
      </FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { createDir, createFile } from '@/api/files'
import FpModal from '@/components/ui/FpModal.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpInput from '@/components/ui/FpInput.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { joinPath } from './fileUtils'

const props = defineProps<{ kind: 'file' | 'dir'; dir: string }>()
const emit = defineEmits<{ done: [] }>()
const visible = defineModel<boolean>({ required: true })

const { t } = useI18n()
const toast = useFpToast()

const name = ref('')
const running = ref(false)

watch(visible, (open) => {
  if (open) name.value = ''
})

async function submit() {
  const trimmed = name.value.trim()
  if (!trimmed || running.value) return
  const path = joinPath(props.dir, trimmed)
  running.value = true
  try {
    await (props.kind === 'dir' ? createDir(path) : createFile(path))
    toast.success(t('common.success'))
    visible.value = false
    emit('done')
  } catch (e) {
    toast.error(e, t('common.failed'))
  } finally {
    running.value = false
  }
}
</script>

<style scoped>
.modal-form {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-2);
}
.hint {
  margin: 0;
  color: var(--fp-text-secondary);
  font-size: 12px;
  word-break: break-all;
}
</style>
