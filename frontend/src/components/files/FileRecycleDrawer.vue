<template>
  <FpDrawer v-model="visible" :header="t('file.recycleBin')" class="recycle-drawer">
    <div class="recycle-toolbar">
      <span class="hint">{{ t('file.recycleHint') }}</span>
      <FpButton
        variant="ghost"
        icon="oi oi-refresh"
        :loading="loading"
        :aria-label="t('common.refresh')"
        :title="t('common.refresh')"
        @click="load"
      />
      <FpButton
        v-permission="{ perm: 'file:write', mode: 'view' }"
        variant="danger"
        icon="oi oi-trash"
        :disabled="!items.length"
        @click="confirmEmpty"
      >
        {{ t('file.emptyRecycle') }}
      </FpButton>
    </div>
    <FpStatePanel
      :loading="loading && !items.length"
      :error="error"
      :empty="!items.length"
      retryable
      :empty-title="t('file.recycleEmpty')"
      @retry="load"
    >
      <ul class="recycle-list">
        <li v-for="it in items" :key="it.id" class="recycle-item">
          <div class="recycle-main">
            <span class="recycle-name">
              <i :class="it.is_dir ? 'oi oi-folder' : 'oi oi-file'" />
              {{ it.name }}
            </span>
            <span class="recycle-meta"> {{ t('file.originalPath') }}: {{ it.original_path }} </span>
            <span class="recycle-meta">
              {{ formatSize(it.size) }}{{ it.size_truncated ? '+' : '' }} ·
              {{ t('file.deletedBy', { user: it.deleted_by }) }} · {{ formatDate(it.deleted_at) }} ·
              {{ t('file.expiresAt', { time: formatDate(it.expires_at) }) }}
            </span>
          </div>
          <div class="recycle-actions">
            <FpButton
              v-permission="'file:write'"
              variant="link"
              :loading="busy === it.id"
              @click="restore(it)"
            >
              {{ t('file.restore') }}
            </FpButton>
            <FpButton
              v-permission="{ perm: 'file:write', mode: 'view' }"
              variant="link"
              class="danger-link"
              @click="confirmPurge(it)"
            >
              {{ t('file.purge') }}
            </FpButton>
          </div>
        </li>
      </ul>
    </FpStatePanel>
  </FpDrawer>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { emptyRecycle, listRecycle, purgeRecycle, restoreRecycle } from '@/api/files'
import type { RecycleItem } from '@/types'
import FpDrawer from '@/components/ui/FpDrawer.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpStatePanel from '@/components/ui/FpStatePanel.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { getErrorMessage } from '@/utils/error'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import { formatSize } from '@/utils/format'
import { errorStatus } from './fileUtils'

const emit = defineEmits<{ restored: [] }>()
const visible = defineModel<boolean>({ required: true })

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

const items = ref<RecycleItem[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const busy = ref('')

async function load() {
  loading.value = true
  error.value = null
  try {
    items.value = (await listRecycle()).data
  } catch (e) {
    error.value = getErrorMessage(e, t('common.failed'))
  } finally {
    loading.value = false
  }
}

watch(visible, (open) => {
  if (open) void load()
})

function formatDate(s: string): string {
  return new Date(s).toLocaleString()
}

async function restore(it: RecycleItem) {
  busy.value = it.id
  try {
    await restoreRecycle(it.id)
    toast.success(t('file.restored', { path: it.original_path }))
    emit('restored')
    await load()
  } catch (e) {
    if (errorStatus(e) === 409) toast.warning(t('file.restoreConflict', { path: it.original_path }))
    else toast.error(e, t('common.failed'))
  } finally {
    busy.value = ''
  }
}

function confirmPurge(it: RecycleItem) {
  confirmAction({
    header: t('common.confirm'),
    message: t('file.purgeConfirm', { name: it.name }),
    rejectLabel: t('file.cancel'),
    accept: async () => {
      try {
        await purgeRecycle(it.id)
        toast.success(t('common.success'))
        await load()
      } catch (e) {
        toast.error(e, t('common.failed'))
      }
    },
  })
}

function confirmEmpty() {
  confirmAction({
    header: t('common.confirm'),
    message: t('file.emptyRecycleConfirm', { n: items.value.length }),
    rejectLabel: t('file.cancel'),
    accept: async () => {
      try {
        await emptyRecycle()
        toast.success(t('common.success'))
        await load()
      } catch (e) {
        toast.error(e, t('common.failed'))
      }
    },
  })
}
</script>

<style scoped>
.recycle-toolbar {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  margin-bottom: var(--fp-space-3);
}
.hint {
  flex: 1;
  color: var(--fp-text-secondary);
  font-size: 12px;
}
.recycle-list {
  list-style: none;
  margin: 0;
  padding: 0;
}
.recycle-item {
  display: flex;
  gap: var(--fp-space-2);
  align-items: flex-start;
  padding: var(--fp-space-2) 0;
  border-bottom: 1px solid var(--fp-border);
}
.recycle-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.recycle-name {
  display: inline-flex;
  align-items: center;
  gap: var(--fp-space-2);
  font-weight: 500;
  overflow-wrap: break-word;
}
.recycle-meta {
  color: var(--fp-text-muted);
  font-size: 12px;
  overflow-wrap: break-word;
}
.recycle-actions {
  display: flex;
  flex-shrink: 0;
}
.danger-link {
  color: var(--fp-danger);
}
</style>
