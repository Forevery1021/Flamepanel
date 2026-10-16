<template>
  <FpModal v-model="visible" :header="mode === 'copy' ? t('file.copyTo') : t('file.moveTo')">
    <form class="modal-form" @submit.prevent="submit">
      <p class="hint">{{ t('file.transferItems', { n: paths.length }) }}</p>
      <FpInput
        v-model="dest"
        :label="t('file.targetDir')"
        :invalid="!dest.trim().startsWith('/')"
        autocomplete="off"
      />
      <FpSwitch v-model="overwrite" :label="t('file.replaceExisting')" />
      <FpInlineMessage v-if="failures.length" severity="warn">
        <div>{{ t('file.transferPartial', { ok: succeeded, failed: failures.length }) }}</div>
        <ul class="failures">
          <li v-for="f in failures" :key="f.source">
            <span class="mono">{{ f.source }}</span>
            <span class="reason">
              {{ f.error_kind === 'conflict' ? t('file.transferConflict') : f.error }}
            </span>
          </li>
        </ul>
      </FpInlineMessage>
    </form>
    <template #footer>
      <FpButton variant="ghost" @click="visible = false">{{ t('file.cancel') }}</FpButton>
      <FpButton
        variant="primary"
        :loading="running"
        :disabled="!dest.trim().startsWith('/') || !paths.length"
        @click="submit"
      >
        {{ mode === 'copy' ? t('file.copy') : t('file.move') }}
      </FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { transferFiles, type TransferItem, type TransferMode } from '@/api/files'
import FpModal from '@/components/ui/FpModal.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import FpInlineMessage from '@/components/ui/FpInlineMessage.vue'
import { useFpToast } from '@/components/ui/FpToast'

const props = defineProps<{ paths: string[]; mode: TransferMode; dir: string }>()
const emit = defineEmits<{ done: [] }>()
const visible = defineModel<boolean>({ required: true })

const { t } = useI18n()
const toast = useFpToast()

const dest = ref('/')
const overwrite = ref(false)
const running = ref(false)
const failures = ref<TransferItem[]>([])
const succeeded = ref(0)

watch(
  visible,
  (open) => {
    if (!open) return
    dest.value = props.dir || '/'
    overwrite.value = false
    failures.value = []
    succeeded.value = 0
  },
  { immediate: true },
)

async function submit() {
  const target = dest.value.trim()
  if (!target.startsWith('/') || !props.paths.length || running.value) return
  running.value = true
  try {
    const { data } = await transferFiles(props.mode, props.paths, target, overwrite.value)
    succeeded.value = data.succeeded
    failures.value = data.items.filter((i) => !i.ok)
    if (data.succeeded > 0) emit('done')
    if (!failures.value.length) {
      toast.success(
        t(props.mode === 'copy' ? 'file.copyDone' : 'file.moveDone', { n: data.succeeded }),
      )
      visible.value = false
    }
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
  gap: var(--fp-space-3);
}
.hint {
  margin: 0;
  color: var(--fp-text-secondary);
}
.failures {
  margin: var(--fp-space-2) 0 0;
  padding-left: var(--fp-space-4);
  max-height: 12rem;
  overflow-y: auto;
}
.failures li {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fp-space-2);
}
.mono {
  font-family: var(--fp-font-mono, monospace);
  word-break: break-all;
}
.reason {
  color: var(--fp-text-secondary);
}
</style>
