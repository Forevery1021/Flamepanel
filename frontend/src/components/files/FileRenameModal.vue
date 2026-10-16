<template>
  <FpModal v-model="visible" :header="t('file.rename')">
    <form class="modal-form" @submit.prevent="submit">
      <FpInput v-model="newName" :label="t('file.newName')" />
    </form>
    <template #footer>
      <FpButton variant="ghost" @click="visible = false">{{ t('file.cancel') }}</FpButton>
      <FpButton variant="primary" :disabled="!newName.trim()" :loading="running" @click="submit">
        {{ t('file.rename') }}
      </FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { renameFile } from '@/api/files'
import FpModal from '@/components/ui/FpModal.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpInput from '@/components/ui/FpInput.vue'
import { useFpToast } from '@/components/ui/FpToast'
import type { FileInfo } from '@/types'
import { joinPath, parentOf } from './fileUtils'

const props = defineProps<{ file: Pick<FileInfo, 'path' | 'name'> | null }>()
const emit = defineEmits<{ done: [] }>()
const visible = defineModel<boolean>({ required: true })

const { t } = useI18n()
const toast = useFpToast()

const newName = ref('')
const running = ref(false)

watch(visible, (open) => {
  if (open) newName.value = props.file?.name ?? ''
})

async function submit() {
  const name = newName.value.trim()
  if (!props.file || !name || running.value) return
  running.value = true
  try {
    await renameFile(props.file.path, joinPath(parentOf(props.file.path), name))
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
</style>
