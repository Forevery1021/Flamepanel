<template>
  <FpModal v-model="visible" :header="t('file.upload')" size="lg" :dismissable-mask="!running">
    <div class="modal-form">
      <p class="upload-target">{{ t('file.uploadTo', { dir }) }}</p>
      <label
        class="drop-zone"
        :class="{ dragging }"
        @dragover.prevent="dragging = true"
        @dragleave.prevent="dragging = false"
        @drop.prevent="onDrop"
      >
        <i class="oi oi-upload" />
        <span>{{ t('file.uploadHere') }}</span>
        <input
          type="file"
          multiple
          class="sr-only"
          :disabled="running"
          :aria-label="t('file.uploadSelect')"
          @change="onPick"
        />
      </label>
      <FpSwitch v-model="overwrite" :label="t('file.overwrite')" :disabled="running" />
      <ul v-if="items.length" class="upload-list">
        <li v-for="(it, i) in items" :key="i" class="upload-item">
          <div class="upload-row">
            <span class="upload-name" :title="it.file.name">{{ it.file.name }}</span>
            <span class="upload-size">{{ formatSize(it.file.size) }}</span>
            <FpTag :value="statusLabel(it)" :severity="statusSeverity(it)" />
            <FpButton
              v-if="!running && it.status !== 'done'"
              variant="link"
              icon="oi oi-times"
              :aria-label="t('file.remove')"
              @click="items.splice(i, 1)"
            />
          </div>
          <FpProgress
            v-if="it.status === 'uploading'"
            :value="it.progress"
            class="upload-progress"
          />
          <p v-if="it.error" class="upload-error">{{ it.error }}</p>
        </li>
      </ul>
    </div>
    <template #footer>
      <FpButton variant="ghost" :disabled="running" @click="visible = false">
        {{ t('file.close') }}
      </FpButton>
      <FpButton variant="primary" :loading="running" :disabled="!pending.length" @click="start">
        {{ t('file.upload') }} ({{ pending.length }})
      </FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { uploadFile } from '@/api/files'
import FpModal from '@/components/ui/FpModal.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpProgress from '@/components/ui/FpProgress.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { getErrorMessage } from '@/utils/error'
import { formatSize } from '@/utils/format'
import { errorStatus } from './fileUtils'

type Status = 'pending' | 'uploading' | 'done' | 'exists' | 'failed'
interface Item {
  file: File
  status: Status
  progress: number
  error: string
}

const props = defineProps<{ dir: string }>()
const emit = defineEmits<{ uploaded: [] }>()
const visible = defineModel<boolean>({ required: true })

const { t } = useI18n()
const toast = useFpToast()

const items = ref<Item[]>([])
const overwrite = ref(false)
const running = ref(false)
const dragging = ref(false)

/** 可（重新）上传的条目：已存在的在勾选「覆盖」后也会重试 */
const pending = computed(() =>
  items.value.filter(
    (it) => it.status === 'pending' || it.status === 'failed' || it.status === 'exists',
  ),
)

watch(visible, (open) => {
  if (open) {
    items.value = []
    overwrite.value = false
  }
})

function add(files: FileList | null | undefined) {
  for (const file of Array.from(files ?? [])) {
    items.value.push({ file, status: 'pending', progress: 0, error: '' })
  }
}

function onPick(e: Event) {
  const input = e.target as HTMLInputElement
  add(input.files)
  input.value = ''
}

function onDrop(e: DragEvent) {
  dragging.value = false
  if (!running.value) add(e.dataTransfer?.files)
}

function statusLabel(it: Item): string {
  if (it.status === 'uploading') return `${it.progress}%`
  return t(`file.uploadStatus.${it.status}`)
}

function statusSeverity(it: Item): 'neutral' | 'info' | 'success' | 'warning' | 'danger' {
  const map = {
    pending: 'neutral',
    uploading: 'info',
    done: 'success',
    exists: 'warning',
    failed: 'danger',
  } as const
  return map[it.status]
}

async function start() {
  running.value = true
  let ok = 0
  for (const it of pending.value.slice()) {
    it.status = 'uploading'
    it.progress = 0
    it.error = ''
    try {
      await uploadFile(props.dir, it.file, {
        overwrite: overwrite.value,
        onProgress: (loaded, total) => {
          it.progress = total ? Math.min(100, Math.round((loaded / total) * 100)) : 0
        },
      })
      it.status = 'done'
      ok++
    } catch (e) {
      if (errorStatus(e) === 409) {
        it.status = 'exists'
        it.error = t('file.uploadExists')
      } else {
        it.status = 'failed'
        it.error = getErrorMessage(e, t('common.failed'))
      }
    }
  }
  running.value = false
  if (ok) {
    toast.success(t('file.uploadDone', { n: ok }))
    emit('uploaded')
  }
  if (items.value.every((it) => it.status === 'done')) visible.value = false
}
</script>

<style scoped>
.modal-form {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
}
.upload-target {
  margin: 0;
  color: var(--fp-text-secondary);
  word-break: break-all;
}
.drop-zone {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--fp-space-2);
  padding: var(--fp-space-5);
  border: 1px dashed var(--fp-border);
  border-radius: var(--fp-radius-md);
  color: var(--fp-text-secondary);
  cursor: pointer;
  text-align: center;
}
.drop-zone:focus-within,
.drop-zone.dragging {
  border-color: var(--fp-brand);
  color: var(--fp-brand);
}
.drop-zone i {
  font-size: 24px;
}
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
  white-space: nowrap;
}
.upload-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-2);
  max-height: 320px;
  overflow: auto;
}
.upload-row {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
}
.upload-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.upload-size {
  color: var(--fp-text-muted);
  font-size: 12px;
}
.upload-progress {
  margin-top: var(--fp-space-1);
}
.upload-error {
  margin: var(--fp-space-1) 0 0;
  color: var(--fp-danger);
  font-size: 12px;
}
</style>
