<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import api from '@/api/client'
import type { AppManifest, InstalledApp } from '@/types'

const catalog = ref<AppManifest[]>([])
const installed = ref<InstalledApp[]>([])
const loading = ref(false)
const search = ref('')
const activeCategory = ref('')
const installDialogVisible = ref(false)
const installManifest = ref<AppManifest | null>(null)
const logsDialogVisible = ref(false)
const logsAppId = ref(0)
const logsAppName = ref('')
const logsContent = ref('')

const installForm = ref({
  name: '',
  port: 0,
  db_host: '127.0.0.1',
  db_port: '3306',
})

const categories = computed(() => {
  const cats = new Set<string>()
  catalog.value.forEach(a => cats.add(a.category))
  return Array.from(cats)
})

const filteredCatalog = computed(() => {
  let items = catalog.value
  if (activeCategory.value) {
    items = items.filter(a => a.category === activeCategory.value)
  }
  if (search.value) {
    const q = search.value.toLowerCase()
    items = items.filter(a =>
      a.name.toLowerCase().includes(q) ||
      a.key.toLowerCase().includes(q) ||
      a.description.toLowerCase().includes(q)
    )
  }
  return items
})

const fetchCatalog = async () => {
  try {
    const res = await api.get<AppManifest[]>('/appstore/catalog')
    catalog.value = res.data
  } catch (e: any) {
    ElMessage.error('加载应用目录失败')
  }
}

const fetchInstalled = async () => {
  loading.value = true
  try {
    const res = await api.get<InstalledApp[]>('/appstore/installed')
    installed.value = res.data
  } catch (e: any) {
    ElMessage.error('加载已安装应用失败')
  } finally {
    loading.value = false
  }
}

const openInstall = (app: AppManifest) => {
  installManifest.value = app
  installForm.value = {
    name: '',
    port: app.default_port,
    db_host: '127.0.0.1',
    db_port: '3306',
  }
  installDialogVisible.value = true
}

const handleInstall = async () => {
  if (!installForm.value.name) {
    ElMessage.warning('请输入应用名称')
    return
  }
  if (!installManifest.value) return

  installDialogVisible.value = false
  ElMessage.info('正在部署应用，请稍候...')
  try {
    const body: any = {
      app_key: installManifest.value.key,
      name: installForm.value.name,
      port: installForm.value.port,
    }
    if (installManifest.value.key === 'phpmyadmin') {
      body.extra_env = {
        db_host: installForm.value.db_host,
        db_port: installForm.value.db_port,
      }
    }
    await api.post('/appstore/install', body)
    ElMessage.success('应用安装成功')
    await fetchInstalled()
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '安装失败')
  }
}

const handleStart = async (app: InstalledApp) => {
  try {
    await api.post(`/appstore/${app.id}/start`)
    ElMessage.success('已启动')
    await fetchInstalled()
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '启动失败')
  }
}

const handleStop = async (app: InstalledApp) => {
  try {
    await api.post(`/appstore/${app.id}/stop`)
    ElMessage.success('已停止')
    await fetchInstalled()
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '停止失败')
  }
}

const handleRestart = async (app: InstalledApp) => {
  try {
    await api.post(`/appstore/${app.id}/restart`)
    ElMessage.success('已重启')
    await fetchInstalled()
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '重启失败')
  }
}

const handleUninstall = async (app: InstalledApp) => {
  try {
    await ElMessageBox.confirm(
      `确定要卸载「${app.name}」吗？所有容器和数据将被永久删除。`,
      '确认卸载',
      { type: 'warning', confirmButtonText: '卸载', confirmButtonClass: 'el-button--danger' }
    )
  } catch { return }
  try {
    await api.delete(`/appstore/${app.id}/uninstall`)
    ElMessage.success('已卸载')
    await fetchInstalled()
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '卸载失败')
  }
}

const showLogs = async (app: InstalledApp) => {
  logsAppId.value = app.id
  logsAppName.value = app.name
  logsDialogVisible.value = true
  try {
    const res = await api.get(`/appstore/${app.id}/logs`)
    logsContent.value = res.data.logs || '（无日志）'
  } catch {
    logsContent.value = '获取日志失败'
  }
}

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

onMounted(() => {
  fetchCatalog()
  fetchInstalled()
})
</script>

<template>
  <div class="appstore-page">
    <div class="page-header">
      <h2>应用商店</h2>
      <p class="desc">一键部署常用应用，基于 Docker Compose 编排</p>
    </div>

    <!-- Installed apps -->
    <div v-if="installed.length > 0" class="section">
      <h3 class="section-title">已安装应用 ({{ installed.length }})</h3>
      <div class="installed-grid">
        <el-card
          v-for="app in installed"
          :key="app.id"
          class="installed-card"
          shadow="hover"
        >
          <div class="inst-top">
            <div class="inst-name">
              <strong>{{ app.name }}</strong>
              <span class="inst-key">{{ app.app_key }}</span>
            </div>
            <el-tag :type="statusTag(app.status)" size="small">
              {{ statusText(app.status) }}
            </el-tag>
          </div>
          <div class="inst-info">
            <span>端口: {{ app.port }}</span>
            <span>版本: {{ app.version }}</span>
          </div>
          <div class="inst-actions">
            <el-button v-if="app.status !== 'running'" size="small" type="primary" @click="handleStart(app)">启动</el-button>
            <el-button v-if="app.status === 'running'" size="small" type="warning" @click="handleStop(app)">停止</el-button>
            <el-button size="small" @click="handleRestart(app)">重启</el-button>
            <el-button size="small" @click="showLogs(app)">日志</el-button>
            <el-button size="small" type="danger" @click="handleUninstall(app)">卸载</el-button>
          </div>
        </el-card>
      </div>
    </div>

    <!-- App catalog -->
    <div class="section">
      <h3 class="section-title">应用目录</h3>
      <div class="catalog-toolbar">
        <el-input
          v-model="search"
          placeholder="搜索应用..."
          clearable
          style="width: 260px"
        />
        <div class="category-filters">
          <el-tag
            :type="activeCategory === '' ? 'primary' : 'info'"
            style="cursor: pointer"
            @click="activeCategory = ''"
          >
            全部
          </el-tag>
          <el-tag
            v-for="cat in categories"
            :key="cat"
            :type="activeCategory === cat ? 'primary' : 'info'"
            style="cursor: pointer"
            @click="activeCategory = cat"
          >
            {{ cat }}
          </el-tag>
        </div>
      </div>

      <div class="catalog-grid">
        <el-card
          v-for="app in filteredCatalog"
          :key="app.key"
          class="catalog-card"
          shadow="hover"
        >
          <div class="catalog-icon">{{ app.icon }}</div>
          <div class="catalog-body">
            <div class="catalog-name">{{ app.name }}</div>
            <div class="catalog-cat">{{ app.category }} · v{{ app.version }}</div>
            <div class="catalog-desc">{{ app.description }}</div>
            <div class="catalog-port">默认端口: {{ app.default_port }}</div>
          </div>
          <el-button type="primary" size="small" @click="openInstall(app)">
            安装
          </el-button>
        </el-card>
      </div>

      <el-empty v-if="filteredCatalog.length === 0" description="没有匹配的应用" />
    </div>

    <!-- Install Dialog -->
    <el-dialog v-model="installDialogVisible" :title="`安装 ${installManifest?.name || ''}`" width="460px" destroy-on-close>
      <el-form :model="installForm" label-width="90px">
        <el-form-item label="实例名称" required>
          <el-input v-model="installForm.name" :placeholder="`例如：my-${installManifest?.key || 'app'}`" />
        </el-form-item>
        <el-form-item label="端口">
          <el-input-number v-model="installForm.port" :min="1024" :max="65535" style="width: 100%" />
        </el-form-item>
        <template v-if="installManifest?.key === 'phpmyadmin'">
          <el-form-item label="数据库地址">
            <el-input v-model="installForm.db_host" placeholder="127.0.0.1" />
          </el-form-item>
          <el-form-item label="数据库端口">
            <el-input v-model="installForm.db_port" placeholder="3306" />
          </el-form-item>
        </template>
      </el-form>
      <template #footer>
        <el-button @click="installDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleInstall">安装</el-button>
      </template>
    </el-dialog>

    <!-- Logs Dialog -->
    <el-dialog v-model="logsDialogVisible" :title="`应用日志 - ${logsAppName}`" width="700px">
      <pre class="logs-pre">{{ logsContent }}</pre>
    </el-dialog>
  </div>
</template>

<style scoped>
.appstore-page {
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

.section {
  margin-top: 24px;
}
.section-title {
  margin: 0 0 14px;
  font-size: 16px;
  color: var(--text-primary);
}

/* Installed */
.installed-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 12px;
}
.installed-card {
  background: var(--bg-card);
  border-color: var(--border-color);
}
.inst-top {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 8px;
}
.inst-name strong {
  color: var(--text-primary);
  font-size: 15px;
}
.inst-key {
  font-size: 11px;
  color: var(--text-secondary);
  margin-left: 8px;
}
.inst-info {
  display: flex;
  gap: 16px;
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 10px;
}
.inst-actions {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

/* Catalog */
.catalog-toolbar {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}
.category-filters {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.catalog-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 14px;
}
.catalog-card {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  background: var(--bg-card);
  border-color: var(--border-color);
}
.catalog-icon {
  font-size: 36px;
  flex-shrink: 0;
}
.catalog-body {
  flex: 1;
  min-width: 0;
}
.catalog-name {
  font-weight: 600;
  color: var(--text-primary);
}
.catalog-cat {
  font-size: 11px;
  color: var(--text-secondary);
  margin: 2px 0 4px;
}
.catalog-desc {
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.5;
  margin-bottom: 4px;
}
.catalog-port {
  font-size: 11px;
  color: var(--text-placeholder);
}

.logs-pre {
  background: #1e1e1e;
  color: #d4d4d4;
  padding: 14px;
  border-radius: 8px;
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 400px;
  overflow: auto;
  margin: 0;
}
</style>
