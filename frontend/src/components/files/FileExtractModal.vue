<template>
  <FpModal v-model="visible" :header="t('file.extract')">
    <div class="modal-form">
      <p class="hint">{{ path }}</p>
      <FpInput v-model="dest" :label="t('file.extractTo')" />
      <FpSwitch v-model="overwrite" :label="t('file.replaceExisting')" />
      <FpInlineMessage v-if="conflict" severity="warn">
        {{ t('file.extractConflict') }}
      </FpInlineMessage>
    </div>
    <template #footer>
      <FpButton variant="ghost" @click="visible = false">{{ t('file.cancel') }}</FpButton>
      <FpButton variant="primary" :loading="running" @click="submit">
        {{ t('file.extract') }}
      </FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { extractArchive } from '@/api/files'
import FpModal from '@/components/ui/FpModal.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import FpInlineMessage from '@/components/ui/FpInlineMessage.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { errorStatus, parentOf } from './fileUtils'

const props = defineProps<{ path: string }>()
const emit = defineEmits<{ done: [] }>()
const visible = defineModel<boolean>({ required: true })

const { t } = useI18n()
const toast = useFpToast()

const dest = ref('/')
const overwrite = ref(false)
const conflict = ref(false)
const running = ref(false)

watch(visible, (open) => {
  if (!open) return
  dest.value = parentOf(props.path)
  overwrite.value = false
  conflict.value = false
})

async function submit() {
  running.value = true
  conflict.value = false
  try {
    const res = await extractArchive(props.path, dest.value.trim() || undefined, overwrite.value)
    toast.success(t('file.extractDone', { n: res.data.entries }))
    visible.value = false
    emit('done')
  } catch (e) {
    if (errorStatus(e) === 409) conflict.value = true
    else toast.error(e, t('common.failed'))
  } finally {
    running.value = false
  }
}
</script>

<style scoped>
.modal-form {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
}
.hint {
  margin: 0;
  color: var(--fp-text-secondary);
  word-break: break-all;
}
</style>
