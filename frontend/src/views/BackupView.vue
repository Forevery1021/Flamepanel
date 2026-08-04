<template>
  <LayoutContent :title="t('backup.title')" reload @reload="fetch">
    <template #toolbar>
      <FpButton variant="primary" icon="oi oi-plus" :loading="creating" @click="onCreate">
        {{ t('backup.create') }}
      </FpButton>
    </template>

    <div class="panel">
      <FpTable :rows="backups" :loading="loading" :paginator="false" :empty-text="t('common.noData')">
        <Column field="filename" :header="t('backup.filename')" style="min-width: 260px" />
        <Column :header="t('backup.size')" style="width: 120px">
          <template #body="{ data }">
            {{ formatSize(data.size) }}
          </template>
        </Column>
        <Column field="created_at" :header="t('backup.createdAt')" style="width: 180px">
          <template #body="{ data }">
            <span class="mono">{{ data.created_at }}</span>
          </template>
        </Column>
        <Column :header="t('common.colActions')" style="width: 240px" frozen>
          <template #body="{ data }">
            <div class="row-actions">
              <FpButton variant="primary" @click="onDownload(data.filename)">
                {{ t('backup.download') }}
              </FpButton>
              <FpButton variant="warning" @click="onRestore(data.filename)">
                {{ t('backup.restore') }}
              </FpButton>
              <FpButton variant="danger" @click="onDelete(data.filename)">
                {{ t('backup.delete') }}
              </FpButton>
            </div>
          </template>
        </Column>
      </FpTable>
    </div>
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import Column from 'openvue/column'
import FpTable from '@/components/ui/FpTable.vue'
import FpButton from '@/components/ui/FpButton.vue'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import { listBackups, createBackup, downloadBackup, restoreBackup, deleteBackup } from '@/api/backups'
import type { BackupEntry } from '@/api/backups'

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

const backups = ref<BackupEntry[]>([])
const loading = ref(false)
const creating = ref(false)

function formatSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

async function fetch() {
  loading.value = true
  try {
    const res = await listBackups()
    backups.value = res.data
  } finally {
    loading.value = false
  }
}

async function onCreate() {
  creating.value = true
  try {
    await createBackup()
    toast.success(t('backup.createSuccess'))
    await fetch()
  } catch (err) {
    toast.error(err, t('common.failed'))
  } finally {
    creating.value = false
  }
}

async function onDownload(filename: string) {
  try {
    await downloadBackup(filename)
  } catch (err) {
    toast.error(err, t('common.failed'))
  }
}

function onRestore(filename: string) {
  confirmAction({
    message: t('backup.restoreConfirm', { name: filename }),
    header: t('common.confirm'),
    acceptLabel: t('backup.restore'),
    rejectLabel: t('common.cancel'),
    accept: async () => {
      try {
        await restoreBackup(filename)
        toast.success(t('backup.restoreSuccess'))
      } catch (err) {
        toast.error(err, t('common.failed'))
      }
    },
  })
}

function onDelete(filename: string) {
  confirmAction({
    message: t('backup.deleteConfirm', { name: filename }),
    header: t('common.confirm'),
    acceptLabel: t('backup.delete'),
    rejectLabel: t('common.cancel'),
    accept: async () => {
      try {
        await deleteBackup(filename)
        toast.success(t('common.success'))
        await fetch()
      } catch (err) {
        toast.error(err, t('common.failed'))
      }
    },
  })
}

onMounted(fetch)
</script>

<style scoped>
.panel {
  padding: var(--fp-space-4);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
}
.row-actions {
  display: flex;
  gap: var(--fp-space-2);
}
</style>
