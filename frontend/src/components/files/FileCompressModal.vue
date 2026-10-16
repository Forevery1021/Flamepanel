<template>
  <FpModal v-model="visible" :header="t('file.compress')">
    <div class="modal-form">
      <p class="hint">{{ t('file.compressItems', { n: paths.length }) }}</p>
      <FpInput v-model="name" :label="t('file.archiveName')" />
      <FpSelectButton
        v-model="format"
        :options="formats"
        option-label="label"
        option-value="value"
        :allow-empty="false"
        :aria-label="t('file.format')"
      />
      <FpSwitch v-model="overwrite" :label="t('file.overwrite')" />
    </div>
    <template #footer>
      <FpButton variant="ghost" @click="visible = false">{{ t('file.cancel') }}</FpButton>
      <FpButton variant="primary" :loading="running" :disabled="!name.trim()" @click="submit">
        {{ t('file.compress') }}
      </FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { compressFiles } from '@/api/files'
import FpModal from '@/components/ui/FpModal.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import FpSelectButton from '@/components/ui/FpSelectButton.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { errorStatus, joinPath, parentOf } from './fileUtils'

const props = defineProps<{ paths: string[] }>()
const emit = defineEmits<{ done: [] }>()
const visible = defineModel<boolean>({ required: true })

const { t } = useI18n()
const toast = useFpToast()

const formats: Array<{ label: string; value: string }> = [
  { label: 'zip', value: '.zip' },
  { label: 'tar.gz', value: '.tar.gz' },
]
const name = ref('')
const format = ref('.zip')
const overwrite = ref(false)
const running = ref(false)

watch(visible, (open) => {
  if (!open) return
  const first = props.paths[0]?.split('/').pop() || 'archive'
  name.value = props.paths.length === 1 ? first : 'archive'
  overwrite.value = false
})

async function submit() {
  if (!props.paths.length) return
  const base = name.value.trim().replace(/\.(zip|tar\.gz|tgz)$/i, '')
  const dest = joinPath(parentOf(props.paths[0]!), base + (format.value || '.zip'))
  running.value = true
  try {
    const res = await compressFiles(props.paths, dest, overwrite.value)
    toast.success(t('file.compressDone', { name: res.data.path.split('/').pop() }))
    visible.value = false
    emit('done')
  } catch (e) {
    if (errorStatus(e) === 409) toast.warning(t('file.archiveExists'))
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
}
</style>
