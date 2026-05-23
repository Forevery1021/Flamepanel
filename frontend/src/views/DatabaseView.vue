<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import api from '@/api/client'
import type { DatabaseInstance, DatabaseBackup } from '@/types'

const instances = ref<DatabaseInstance[]>([])
const backups = ref<DatabaseBackup[]>([])
const loading = ref(false)
const dialogVisible = ref(false)
const backupDrawerVisible = ref(false)
const backupInstanceId = ref(0)
const backupInstanceName = ref('')

const dbTypeOptions = [
  { value: 'mysql', label: 'MySQL', icon: '🐬', defaultPort: 3306 },
  { value: 'mariadb', label: 'MariaDB', icon: '🦭', defaultPort: 3306 },
  { value: 'postgresql', label: 'PostgreSQL', icon: '🐘', defaultPort: 5432 },
  { value: 'redis', label: 'Redis', icon: '🔴', defaultPort: 6379 },
  { value: 'mongodb', label: 'MongoDB', icon: '🍃', defaultPort: 27017 },
]

const form = ref({
  name: '',
  db_type: 'mysql',
  version: 'latest',
  port: 3306,
  password: '',
})

const typeLabel = (t: string) => dbTypeOptions.find(d => d.value === t)?.label || t
const typeIcon = (t: string) => dbTypeOptions.find(d => d.value === t)?.icon || '📦'

const statusTag = (s: string) => {
  const map: Record<string, string> = {
    running: 'success', stopped: 'info', error: 'danger', installing: 'warning'
  }
  return map[s] || 'info'
}
const statusText = (s: string) => {
  const map: Record<string, string> = {
    running: '运行中', stopped: '已停止', error: '错误', installing: '安装中'
  }
  return map[s] || s
}

const fetchInstances = async () => {
  loading.value = true
  try {
    const res = await api.get<DatabaseInstance[]>('/databases')
    instances.value = res.data
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '加载失败')
  } finally {
    loading.value = false
  }
}

const openCreate = () => {
  form.value = { name: '', db_type: 'mysql', version: 'latest', port: 3306, password: '' }
  dialogVisible.value = true
}

const onTypeChange = (type: string) => {
  const opt = dbTypeOptions.find(d => d.value === type)
  if (opt) form.value.port = opt.defaultPort
}

const handleCreate = async () => {
  if (!form.value.name || !form.value.password) {
    ElMessage.warning('请填写名称和密码')
    return
  }
  dialogVisible.value = false
  ElMessage.info('正在拉取镜像并创建数据库，请稍候...')
  try {
    await api.post('/databases', {
      name: form.value.name,
      db_type: form.value.db_type,
      version: form.value.version || undefined,
      port: form.value.port,
      password: form.value.password,
    })
    ElMessage.success('数据库创建成功')
    await fetchInstances()
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '创建失败')
  }
}

const handleStart = async (inst: DatabaseInstance) => {
  try {
    await api.post(`/databases/${inst.id}/start`)
    ElMessage.success('已启动')
    inst.status = 'running'
    await fetchInstances()
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '启动失败')
  }
}

const handleStop = async (inst: DatabaseInstance) => {
  try {
    await api.post(`/databases/${inst.id}/stop`)
    ElMessage.success('已停止')
    inst.status = 'stopped'
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '停止失败')
  }
}

const handleDelete = async (inst: DatabaseInstance) => {
  try {
    await ElMessageBox.confirm(
      `确定要删除「${inst.name}」吗？容器和数据目录将被永久删除。`,
      '确认删除',
      { type: 'warning', confirmButtonText: '删除', confirmButtonClass: 'el-button--danger' }
    )
  } catch {
    return
  }
  try {
    await api.delete(`/databases/${inst.id}`)
    ElMessage.success('已删除')
    await fetchInstances()
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '删除失败')
  }
}

const handleBackup = async (inst: DatabaseInstance) => {
  if (inst.status !== 'running') {
    ElMessage.warning('数据库未运行，无法备份')
    return
  }
  try {
    ElMessage.info('正在备份，请稍候...')
    await api.post(`/databases/${inst.id}/backup`)
    ElMessage.success('备份完成')
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '备份失败')
  }
}

const showBackups = async (inst: DatabaseInstance) => {
  backupInstanceId.value = inst.id
  backupInstanceName.value = inst.name
  backupDrawerVisible.value = true
  try {
    const res = await api.get<DatabaseBackup[]>(`/databases/${inst.id}/backups`)
    backups.value = res.data
  } catch {
    backups.value = []
  }
}

const formatSize = (bytes: number) => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i]
}

const connString = (inst: DatabaseInstance) => {
  const host = window.location.hostname
  switch (inst.db_type) {
    case 'redis': return `redis://:${inst.password}@${host}:${inst.port}`
    case 'mongodb': return `mongodb://root:${inst.password}@${host}:${inst.port}`
    case 'postgresql': return `postgresql://root:${inst.password}@${host}:${inst.port}/postgres`
    default: return `${inst.db_type}://root:${inst.password}@${host}:${inst.port}`
  }
}

onMounted(() => {
  fetchInstances()
})
</script>

<template>
  <div class="database-page">
    <div class="page-header">
      <h2>数据库管理</h2>
      <p class="desc">一键部署和管理 MySQL、PostgreSQL、Redis、MongoDB 数据库</p>
    </div>

    <div class="toolbar">
      <el-button type="primary" @click="openCreate">创建数据库</el-button>
      <el-button :loading="loading" @click="fetchInstances">刷新</el-button>
    </div>

    <!-- Instance cards -->
    <div v-loading="loading" class="instance-grid">
      <el-empty v-if="instances.length === 0 && !loading" description="暂无数据库实例" />

      <el-card
        v-for="inst in instances"
        :key="inst.id"
        class="instance-card"
        shadow="hover"
      >
        <div class="card-top">
          <div class="card-title">
            <span class="db-icon">{{ typeIcon(inst.db_type) }}</span>
            <div>
              <div class="db-name">{{ inst.name }}</div>
              <div class="db-type">{{ typeLabel(inst.db_type) }} {{ inst.version }}</div>
            </div>
          </div>
          <el-tag :type="statusTag(inst.status)" size="small">
            {{ statusText(inst.status) }}
          </el-tag>
        </div>

        <div class="card-info">
          <div class="info-row">
            <span class="info-label">端口</span>
            <span class="info-value">{{ inst.port }}</span>
          </div>
          <div class="info-row">
            <span class="info-label">连接串</span>
            <span class="info-value mono">{{ connString(inst) }}</span>
          </div>
          <div class="info-row">
            <span class="info-label">创建时间</span>
            <span class="info-value">{{ inst.created_at }}</span>
          </div>
        </div>

        <div class="card-actions">
          <el-button
            v-if="inst.status !== 'running'"
            size="small"
            type="primary"
            @click="handleStart(inst)"
          >
            启动
          </el-button>
          <el-button
            v-if="inst.status === 'running'"
            size="small"
            type="warning"
            @click="handleStop(inst)"
          >
            停止
          </el-button>
          <el-button size="small" @click="handleBackup(inst)">
            备份
          </el-button>
          <el-button size="small" @click="showBackups(inst)">
            备份记录
          </el-button>
          <el-button
            size="small"
            type="danger"
            @click="handleDelete(inst)"
          >
            删除
          </el-button>
        </div>
      </el-card>
    </div>

    <!-- Create Dialog -->
    <el-dialog v-model="dialogVisible" title="创建数据库" width="500px" destroy-on-close>
      <el-form :model="form" label-width="90px">
        <el-form-item label="数据库类型" required>
          <el-select v-model="form.db_type" @change="onTypeChange" style="width: 100%">
            <el-option
              v-for="opt in dbTypeOptions"
              :key="opt.value"
              :label="`${opt.icon} ${opt.label}`"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="实例名称" required>
          <el-input v-model="form.name" placeholder="例如：my-project-db" />
        </el-form-item>
        <el-form-item label="版本">
          <el-input v-model="form.version" placeholder="latest" />
        </el-form-item>
        <el-form-item label="端口">
          <el-input-number v-model="form.port" :min="1024" :max="65535" style="width: 100%" />
        </el-form-item>
        <el-form-item label="密码" required>
          <el-input
            v-model="form.password"
            type="password"
            show-password
            placeholder="设置 root 密码"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleCreate">创建</el-button>
      </template>
    </el-dialog>

    <!-- Backups Drawer -->
    <el-drawer
      v-model="backupDrawerVisible"
      :title="`备份记录 - ${backupInstanceName}`"
      size="400px"
    >
      <el-empty v-if="backups.length === 0" description="暂无备份记录" />
      <el-table v-else :data="backups" size="small">
        <el-table-column prop="filename" label="文件名" min-width="180" show-overflow-tooltip />
        <el-table-column label="大小" width="80">
          <template #default="{ row }">
            {{ formatSize(row.size_bytes) }}
          </template>
        </el-table-column>
        <el-table-column prop="created_at" label="时间" width="160" />
      </el-table>
    </el-drawer>
  </div>
</template>

<style scoped>
.database-page {
  padding: 24px;
  max-width: 1200px;
}

.page-header h2 {
  margin: 0;
  font-size: 22px;
  color: var(--text-primary);
}
.page-header .desc {
  margin: 4px 0 0;
  color: var(--text-secondary);
  font-size: 13px;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin: 20px 0;
}

.instance-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
  gap: 16px;
}

.instance-card {
  background: var(--bg-card);
  border-color: var(--border-color);
}

.card-top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: 16px;
}

.card-title {
  display: flex;
  align-items: center;
  gap: 10px;
}
.db-icon {
  font-size: 28px;
}
.db-name {
  font-weight: 600;
  font-size: 16px;
  color: var(--text-primary);
}
.db-type {
  font-size: 12px;
  color: var(--text-secondary);
}

.card-info {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 14px;
}
.info-row {
  display: flex;
  gap: 10px;
  font-size: 13px;
}
.info-label {
  color: var(--text-secondary);
  min-width: 60px;
}
.info-value {
  color: var(--text-primary);
}
.info-value.mono {
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 11px;
  word-break: break-all;
}

.card-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
</style>
