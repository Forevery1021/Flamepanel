<template>
  <FpModal v-model="visible" :header="header" size="xl" :dismissable-mask="false">
    <FpStatePanel :loading="loading" :error="loadError" retryable @retry="load">
      <FpTextarea
        v-model="content"
        rows="22"
        class="editor mono w-full"
        spellcheck="false"
        :aria-label="t('file.content')"
        @keydown="onKeydown"
      />
      <p class="editor-hint">{{ t('file.saveShortcut') }}</p>
    </FpStatePanel>
    <template #footer>
      <FpButton variant="ghost" @click="requestClose">{{ t('file.close') }}</FpButton>
      <FpButton
        v-permission="'file:write'"
        variant="primary"
        :loading="saving"
        :disabled="loading || !!loadError || !dirty"
        @click="save()"
      >
        {{ t('file.save') }}
      </FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { readFile, writeFile } from '@/api/files'
import type { FileInfo } from '@/types'
import FpModal from '@/components/ui/FpModal.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTextarea from '@/components/ui/FpTextarea.vue'
import FpStatePanel from '@/components/ui/FpStatePanel.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { getErrorMessage } from '@/utils/error'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import { errorStatus } from './fileUtils'

const props = defineProps<{ file: FileInfo | null }>()
const emit = defineEmits<{ saved: [] }>()
const visible = defineModel<boolean>({ required: true })

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

const content = ref('')
const original = ref('')
/** 打开 / 上次保存时文件的修改时间：保存时交给后端做冲突检测 */
const baseModified = ref<string | undefined>()
const loading = ref(false)
const loadError = ref<string | null>(null)
const saving = ref(false)

const dirty = computed(() => content.value !== original.value)
const header = computed(() => {
  const name = props.file?.path ?? ''
  return dirty.value ? `${name} · ${t('file.unsaved')}` : name
})

async function load() {
  if (!props.file) return
  loading.value = true
  loadError.value = null
  try {
    baseModified.value = props.file.modified_at
    const res = await readFile(props.file.path)
    content.value = res.data
    original.value = res.data
  } catch (e) {
    loadError.value = getErrorMessage(e, t('common.failed'))
  } finally {
    loading.value = false
  }
}

watch(
  () => [visible.value, props.file?.path] as const,
  ([open]) => {
    if (open) void load()
  },
  { immediate: true },
)

async function save(force = false) {
  if (!props.file || saving.value || !dirty.value) return
  saving.value = true
  try {
    const text = content.value
    const res = await writeFile(props.file.path, text, force ? undefined : baseModified.value)
    baseModified.value = res.data.modified_at
    original.value = text
    toast.success(t('common.success'))
    emit('saved')
  } catch (e) {
    if (errorStatus(e) === 409) {
      confirmAction({
        header: t('common.confirm'),
        message: t('file.editConflict'),
        acceptLabel: t('file.overwrite'),
        rejectLabel: t('file.cancel'),
        accept: () => void save(true),
      })
    } else {
      toast.error(e, t('common.failed'))
    }
  } finally {
    saving.value = false
  }
}

function onKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
    e.preventDefault()
    void save()
  }
}

function requestClose() {
  if (!dirty.value) {
    visible.value = false
    return
  }
  confirmAction({
    header: t('common.confirm'),
    message: t('file.discardConfirm'),
    rejectLabel: t('file.cancel'),
    accept: () => {
      content.value = original.value
      visible.value = false
    },
  })
}
</script>

<style scoped>
.editor :deep(textarea),
textarea.editor {
  min-height: 60vh;
  tab-size: 4;
  white-space: pre;
  overflow: auto;
}
.editor-hint {
  margin: var(--fp-space-2) 0 0;
  color: var(--fp-text-secondary);
  font-size: 12px;
}
.mono {
  font-family: var(--fp-font-mono);
}
</style>
