<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Refresh, Delete, VideoPlay, Upload } from '@element-plus/icons-vue'
import type {
  BackupConfig, BackupRecord, CreateBackupConfigRequest, UpdateBackupConfigRequest,
} from '@/types'

const configs = ref<BackupConfig[]>([])
const records = ref<BackupRecord[]>([])
const loading = ref(false)
const recordLoading = ref(false)
const storagePlaceholder = computed(() => {
  switch (form.value.storage_type) {
    case 's3': return '{"endpoint":"http://minio:9000","bucket":"backups","access_key":"...","secret_key":"..."}'
    case 'oss': return '{"endpoint":"oss-cn-hangzhou.aliyuncs.com","bucket":"backups","access_key_id":"...","access_key_secret":"..."}'
    default: return 'data/backups'
  }
})

const selectedConfig = ref<BackupConfig | null>(null)

// Dialog state
const dialogVisible = ref(false)
const dialogTitle = ref('新建备份配置')
const editingId = ref<number | null>(null)
const form = ref<CreateBackupConfigRequest>({
  name: '',
  backup_type: 'full',
  target_path: '',
  storage_type: 'local',
  storage_path: 'data/backups',
  cron_expr: '',
  retention_days: 30,
})

// Restore dialog
const restoreVisible = ref(false)
const restoreRecordId = ref<number | null>(null)
const restorePath = ref('')

// Helpers
const API = '/api/backup'

async function loadConfigs() {
  loading.value = true
  try {
    const resp = await fetch(API + '/configs')
    if (!resp.ok) throw new Error('加载失败')
    configs.value = await resp.json()
  } catch (e: any) {
    ElMessage.error('加载备份配置失败: ' + e.message)
  } finally {
    loading.value = false
  }
}

async function loadRecords(configId: number) {
  recordLoading.value = true
  try {
    const resp = await fetch(`${API}/records/${configId}`)
    if (!resp.ok) throw new Error('加载失败')
    records.value = await resp.json()
  } catch (e: any) {
    ElMessage.error('加载备份记录失败: ' + e.message)
  } finally {
    recordLoading.value = false
  }
}

function selectConfig(config: BackupConfig) {
  selectedConfig.value = config
  loadRecords(config.id)
}

function openCreate() {
  dialogTitle.value = '新建备份配置'
  editingId.value = null
  form.value = {
    name: '',
    backup_type: 'full',
    target_path: '',
    storage_type: 'local',
    storage_path: 'data/backups',
    cron_expr: '',
    retention_days: 30,
  }
  dialogVisible.value = true
}

function openEdit(config: BackupConfig) {
  dialogTitle.value = '编辑备份配置'
  editingId.value = config.id
  form.value = {
    name: config.name,
    backup_type: config.backup_type,
    target_path: config.target_path,
    storage_type: config.storage_type,
    storage_path: config.storage_path,
    cron_expr: config.cron_expr || '',
    retention_days: config.retention_days,
  }
  dialogVisible.value = true
}

async function handleSave() {
  if (!form.value.name || !form.value.target_path) {
    ElMessage.warning('请填写名称和目标路径')
    return
  }
  try {
    let resp: Response
    if (editingId.value) {
      resp = await fetch(`${API}/configs/${editingId.value}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(form.value),
      })
    } else {
      resp = await fetch(`${API}/configs`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(form.value),
      })
    }
    if (!resp.ok) {
      const err = await resp.text()
      throw new Error(err)
    }
    ElMessage.success(editingId.value ? '更新成功' : '创建成功')
    dialogVisible.value = false
    loadConfigs()
  } catch (e: any) {
    ElMessage.error('保存失败: ' + e.message)
  }
}

async function handleDelete(config: BackupConfig) {
  try {
    await ElMessageBox.confirm(`确定删除备份配置 "${config.name}"？`, '确认删除', {
      type: 'warning',
    })
  } catch {
    return
  }
  try {
    const resp = await fetch(`${API}/configs/${config.id}`, { method: 'DELETE' })
    if (!resp.ok) throw new Error('删除失败')
    ElMessage.success('删除成功')
    if (selectedConfig.value?.id === config.id) {
      selectedConfig.value = null
      records.value = []
    }
    loadConfigs()
  } catch (e: any) {
    ElMessage.error('删除失败: ' + e.message)
  }
}

async function handleExecute(config: BackupConfig) {
  try {
    await ElMessageBox.confirm(`立即执行备份 "${config.name}"？`, '确认执行', {
      type: 'info',
    })
  } catch {
    return
  }
  try {
    const resp = await fetch(`${API}/execute/${config.id}`, { method: 'POST' })
    if (!resp.ok) {
      const err = await resp.text()
      throw new Error(err)
    }
    ElMessage.success('备份执行完成')
    if (selectedConfig.value?.id === config.id) {
      loadRecords(config.id)
    }
  } catch (e: any) {
    ElMessage.error('备份执行失败: ' + e.message)
  }
}

function openRestore(record: BackupRecord) {
  restoreRecordId.value = record.id
  restorePath.value = ''
  restoreVisible.value = true
}

async function handleRestore() {
  if (!restoreRecordId.value) return
  try {
    const body: any = {}
    if (restorePath.value) {
      body.target_path = restorePath.value
    }
    const resp = await fetch(`${API}/restore/${restoreRecordId.value}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    })
    if (!resp.ok) {
      const err = await resp.text()
      throw new Error(err)
    }
    ElMessage.success('恢复成功')
    restoreVisible.value = false
  } catch (e: any) {
    ElMessage.error('恢复失败: ' + e.message)
  }
}

function formatSize(bytes: number): string {
  if (bytes === 0) return '-'
  const units = ['B', 'KB', 'MB', 'GB']
  let size = bytes
  let idx = 0
  while (size >= 1024 && idx < units.length - 1) {
    size /= 1024
    idx++
  }
  return `${size.toFixed(2)} ${units[idx]}`
}

function statusTag(status: string) {
  switch (status) {
    case 'success': return 'success'
    case 'failed': return 'danger'
    case 'running': return 'warning'
    default: return 'info'
  }
}

function statusText(status: string) {
  switch (status) {
    case 'success': return '成功'
    case 'failed': return '失败'
    case 'running': return '运行中'
    default: return status
  }
}

onMounted(() => {
  loadConfigs()
})
</script>

<template>
  <div class="backup-view">
    <div class="page-header">
      <h2>备份管理</h2>
      <el-button type="primary" :icon="Plus" @click="openCreate">新建配置</el-button>
    </div>

    <div class="content-grid">
      <!-- Config list -->
      <div class="panel config-panel">
        <div class="panel-header">
          <span class="panel-title">备份配置</span>
          <el-button size="small" :icon="Refresh" @click="loadConfigs" :loading="loading" />
        </div>
        <div class="panel-body">
          <div
            v-for="c in configs"
            :key="c.id"
            class="config-item"
            :class="{ active: selectedConfig?.id === c.id }"
            @click="selectConfig(c)"
          >
            <div class="config-main">
              <span class="config-name">{{ c.name }}</span>
              <span class="config-path">{{ c.target_path }}</span>
            </div>
            <div class="config-meta">
              <el-tag :type="c.enabled ? 'success' : 'info'" size="small">
                {{ c.enabled ? '启用' : '禁用' }}
              </el-tag>
              <span class="config-type">{{ c.backup_type }}</span>
            </div>
            <div class="config-actions" @click.stop>
              <el-button size="small" text type="primary" @click="handleExecute(c)">执行</el-button>
              <el-button size="small" text @click="openEdit(c)">编辑</el-button>
              <el-button size="small" text type="danger" @click="handleDelete(c)">删除</el-button>
            </div>
          </div>
          <el-empty v-if="!loading && configs.length === 0" description="暂无备份配置" />
        </div>
      </div>

      <!-- Records list -->
      <div class="panel record-panel">
        <div class="panel-header">
          <span class="panel-title">
            备份记录
            <template v-if="selectedConfig"> — {{ selectedConfig.name }}</template>
          </span>
          <el-button
            v-if="selectedConfig"
            size="small"
            :icon="Refresh"
            @click="loadRecords(selectedConfig.id)"
            :loading="recordLoading"
          />
        </div>
        <div class="panel-body">
          <el-empty v-if="!selectedConfig" description="请选择一个备份配置" />
          <el-empty v-else-if="!recordLoading && records.length === 0" description="暂无备份记录" />
          <div v-else class="record-list">
            <div v-for="r in records" :key="r.id" class="record-item">
              <div class="record-main">
                <span class="record-file">{{ r.file_name }}</span>
                <div class="record-tags">
                  <el-tag :type="statusTag(r.status)" size="small">{{ statusText(r.status) }}</el-tag>
                  <span class="record-size">{{ formatSize(r.file_size) }}</span>
                </div>
              </div>
              <div class="record-meta">
                <span>{{ r.started_at }}</span>
                <span v-if="r.finished_at">完成: {{ r.finished_at }}</span>
              </div>
              <div v-if="r.error_message" class="record-error">{{ r.error_message }}</div>
              <div class="record-actions">
                <el-button
                  v-if="r.status === 'success'"
                  size="small"
                  text
                  type="primary"
                  @click="openRestore(r)"
                >
                  恢复
                </el-button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Create/Edit Dialog -->
    <el-dialog v-model="dialogVisible" :title="dialogTitle" width="560px">
      <el-form :model="form" label-width="100px">
        <el-form-item label="名称" required>
          <el-input v-model="form.name" placeholder="备份配置名称" />
        </el-form-item>
        <el-form-item label="备份类型">
          <el-select v-model="form.backup_type" style="width: 100%">
            <el-option label="完整备份 (full)" value="full" />
            <el-option label="增量备份 (incremental)" value="incremental" />
          </el-select>
        </el-form-item>
        <el-form-item label="目标路径" required>
          <el-input v-model="form.target_path" placeholder="/path/to/backup" />
        </el-form-item>
        <el-form-item label="存储类型">
          <el-select v-model="form.storage_type" style="width: 100%">
            <el-option label="本地" value="local" />
            <el-option label="Amazon S3 / MinIO" value="s3" />
            <el-option label="阿里云 OSS" value="oss" />
          </el-select>
        </el-form-item>
        <el-form-item label="存储配置">
          <el-input
            v-model="form.storage_path"
            :placeholder="storagePlaceholder"
          />
        </el-form-item>
        <el-form-item label="Cron 表达式">
          <el-input v-model="form.cron_expr" placeholder="0 2 * * * (每天凌晨2点)" />
        </el-form-item>
        <el-form-item label="保留天数">
          <el-input-number v-model="form.retention_days" :min="1" :max="365" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSave">保存</el-button>
      </template>
    </el-dialog>

    <!-- Restore Dialog -->
    <el-dialog v-model="restoreVisible" title="恢复备份" width="480px">
      <p>确定要恢复此备份吗？这将会覆盖目标路径中的现有文件。</p>
      <el-form label-width="100px" style="margin-top: 16px">
        <el-form-item label="目标路径 (可选)">
          <el-input v-model="restorePath" placeholder="留空则使用原路径" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="restoreVisible = false">取消</el-button>
        <el-button type="primary" @click="handleRestore">确认恢复</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.backup-view {
  padding: 24px;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.page-header h2 {
  margin: 0;
  font-size: 20px;
  color: var(--text-primary);
}

.content-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
  flex: 1;
  min-height: 0;
}

.panel {
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 10px;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border-color);
}

.panel-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
}

.panel-body {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
}

.config-item {
  padding: 12px 14px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
  margin-bottom: 6px;
  border: 1px solid transparent;
}

.config-item:hover {
  background: var(--bg-hover);
}

.config-item.active {
  background: var(--bg-active, #e8f0fe);
  border-color: #409eff;
}

.config-main {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 8px;
}

.config-name {
  font-weight: 600;
  color: var(--text-primary);
}

.config-path {
  font-size: 12px;
  color: var(--text-secondary);
  font-family: monospace;
}

.config-meta {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 6px;
}

.config-type {
  font-size: 12px;
  color: var(--text-secondary);
}

.config-actions {
  display: flex;
  gap: 4px;
}

.record-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.record-item {
  padding: 12px 14px;
  border-radius: 8px;
  border: 1px solid var(--border-color);
}

.record-main {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}

.record-file {
  font-family: monospace;
  font-size: 13px;
  color: var(--text-primary);
}

.record-tags {
  display: flex;
  align-items: center;
  gap: 8px;
}

.record-size {
  font-size: 12px;
  color: var(--text-secondary);
}

.record-meta {
  font-size: 12px;
  color: var(--text-secondary);
  display: flex;
  gap: 12px;
}

.record-error {
  margin-top: 6px;
  font-size: 12px;
  color: #f56c6c;
  background: #fef0f0;
  padding: 6px 8px;
  border-radius: 4px;
}

.record-actions {
  margin-top: 6px;
}
</style>
