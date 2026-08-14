<template>
  <LayoutContent :title="t('backup.title')" reload @reload="invalidate">
    <template #toolbar>
      <FpButton v-permission="{ perm: 'backup:create', mode: 'view' }" variant="primary" icon="oi oi-plus" :loading="creating" @click="onCreate">
        {{ t('backup.create') }}
      </FpButton>
    </template>

    <div class="panel">
      <FpTable :rows="backups" :loading="loading" :paginator="false" :empty-text="t('common.noData')">
        <FpColumn field="filename" :header="t('backup.filename')" style="min-width: 260px" />
        <FpColumn :header="t('backup.size')" style="width: 120px">
          <template #body="{ data }">
            {{ formatSize(data.size) }}
          </template>
        </FpColumn>
        <FpColumn field="created_at" :header="t('backup.createdAt')" style="width: 180px">
          <template #body="{ data }">
            <span class="mono">{{ data.created_at }}</span>
          </template>
        </FpColumn>
        <FpColumn :header="t('common.colActions')" style="width: 240px" frozen>
          <template #body="{ data }">
            <div class="row-actions">
              <FpButton variant="primary" @click="onDownload(data.filename)">
                {{ t('backup.download') }}
              </FpButton>
              <FpButton v-permission="{ perm: 'backup:update', mode: 'view' }" variant="warning" @click="onRestore(data.filename)">
                {{ t('backup.restore') }}
              </FpButton>
              <FpButton v-permission="{ perm: 'backup:delete', mode: 'view' }" variant="danger" @click="onDelete(data.filename)">
                {{ t('backup.delete') }}
              </FpButton>
            </div>
          </template>
        </FpColumn>
      </FpTable>
    </div>
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpTable from '@/components/ui/FpTable.vue'
import FpButton from '@/components/ui/FpButton.vue'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import { listBackups, createBackup, downloadBackup, restoreBackup, deleteBackup } from '@/api/backups'
import type { BackupEntry } from '@/api/backups'
import { useApiQuery, useQueryCacheClient } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

const queryClient = useQueryCacheClient()
const creating = ref(false)

function formatSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

// P3-A：备份列表走统一数据获取层 useApiQuery
const backupsQuery = useApiQuery<BackupEntry[]>(
  () => queryKeys.backups.list(),
  async () => {
    const res = await listBackups()
    return { data: res.data }
  },
)
const backups = computed<BackupEntry[]>(() => backupsQuery.data.value ?? [])
const loading = backupsQuery.loading

function invalidate() {
  queryClient.invalidateQueries({ queryKey: queryKeys.backups.all })
}

async function onCreate() {
  creating.value = true
  try {
    await createBackup()
    toast.success(t('backup.createSuccess'))
    invalidate()
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
        invalidate()
      } catch (err) {
        toast.error(err, t('common.failed'))
      }
    },
  })
}

</script>

<style scoped>
.row-actions {
  display: flex;
  gap: var(--fp-space-2);
}
</style>
