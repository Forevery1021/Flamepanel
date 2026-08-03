<template>
  <div class="view-container">
    <div class="card-header-title">
      <h2>{{ t('nav.backups') }}</h2>
      <el-button type="primary" :loading="creating" @click="onCreate">
        {{ t('backup.create') }}
      </el-button>
    </div>

    <el-card shadow="hover">
      <el-table
        v-loading="loading"
        :empty-text="t('common.noData')"
        :data="backups"
        border
        stripe
        max-height="620px"
      >
        <el-table-column prop="filename" :label="t('backup.filename')" min-width="260" />
        <el-table-column :label="t('backup.size')" width="120">
          <template #default="{ row }">
            {{ formatSize(row.size) }}
          </template>
        </el-table-column>
        <el-table-column prop="created_at" :label="t('backup.createdAt')" width="180" />
        <el-table-column :label="t('common.colActions')" width="240" fixed="right">
          <template #default="{ row }">
            <el-button size="small" @click="onDownload(row.filename)">
              {{ t('backup.download') }}
            </el-button>
            <el-button
              size="small"
              type="warning"
              plain
              @click="onRestore(row.filename)"
            >
              {{ t('backup.restore') }}
            </el-button>
            <el-button size="small" type="danger" @click="onDelete(row.filename)">
              {{ t('backup.delete') }}
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { listBackups, createBackup, downloadBackup, restoreBackup, deleteBackup } from '@/api/backups'
import type { BackupEntry } from '@/api/backups'

const { t } = useI18n()
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
    ElMessage.success(t('backup.createSuccess'))
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    creating.value = false
  }
}

async function onDownload(filename: string) {
  try {
    await downloadBackup(filename)
  } catch {
    ElMessage.error(t('common.failed'))
  }
}

async function onRestore(filename: string) {
  try {
    await ElMessageBox.confirm(t('backup.restoreConfirm', { name: filename }), t('common.confirm'), {
      type: 'warning',
      confirmButtonText: t('backup.restore'),
      cancelButtonText: t('common.cancel'),
    })
    await restoreBackup(filename)
    ElMessage.success(t('backup.restoreSuccess'))
  } catch {
    // cancelled
  }
}

async function onDelete(filename: string) {
  try {
    await ElMessageBox.confirm(t('backup.deleteConfirm', { name: filename }), t('common.confirm'), {
      type: 'warning',
      confirmButtonText: t('backup.delete'),
      cancelButtonText: t('common.cancel'),
    })
    await deleteBackup(filename)
    ElMessage.success(t('common.success'))
    await fetch()
  } catch {
    // cancelled
  }
}

onMounted(fetch)
</script>
