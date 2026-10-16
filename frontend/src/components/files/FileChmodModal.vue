<template>
  <FpModal v-model="visible" :header="t('file.chmod')">
    <form class="modal-form" @submit.prevent="submit">
      <FpInput v-model="mode" :label="t('file.mode')" placeholder="755" />
    </form>
    <template #footer>
      <FpButton variant="ghost" @click="visible = false">{{ t('file.cancel') }}</FpButton>
      <FpButton variant="primary" :disabled="!mode.trim()" :loading="running" @click="submit">
        {{ t('common.confirm') }}
      </FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { chmodFile } from '@/api/files'
import FpModal from '@/components/ui/FpModal.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpInput from '@/components/ui/FpInput.vue'
import { useFpToast } from '@/components/ui/FpToast'
import type { FileInfo } from '@/types'

const props = defineProps<{ file: Pick<FileInfo, 'path' | 'permissions'> | null }>()
const emit = defineEmits<{ done: [] }>()
const visible = defineModel<boolean>({ required: true })

const { t } = useI18n()
const toast = useFpToast()

const mode = ref('755')
const running = ref(false)

watch(visible, (open) => {
  if (open) mode.value = props.file?.permissions || '755'
})

async function submit() {
  const value = mode.value.trim()
  if (!props.file || !value || running.value) return
  running.value = true
  try {
    await chmodFile(props.file.path, value)
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
