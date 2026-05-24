<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Refresh, Delete, VideoPlay, FolderOpened } from '@element-plus/icons-vue'
import type { NodeInfo } from '@/types'

// ── Types ──────────────────────────────────────────────────────────────────────

interface ClusterDashboard {
  total_nodes: number
  online_nodes: number
  offline_nodes: number
  avg_cpu: number
  avg_memory: number
  avg_disk: number
  avg_load: number
  nodes: NodeInfo[]
}

interface NodeExecResponse {
  node_id: number
  node_name: string
  output: string
  exit_code: number
  duration_ms: number
}

interface FileEntry {
  name: string
  is_dir: boolean
  size: number
  modified: string
}

// ── State ──────────────────────────────────────────────────────────────────────

const API = '/api/nodes'
const nodes = ref<NodeInfo[]>([])
const dashboard = ref<ClusterDashboard | null>(null)
const loading = ref(false)
const selectedNodes = ref<number[]>([])

// Exec dialog
const execDialog = ref(false)
const execMode = ref<'single' | 'batch'>('single')
const execNodeId = ref<number | null>(null)
const execCommand = ref('')
const execTimeout = ref(30)
const execResults = ref<NodeExecResponse[]>([])
const execRunning = ref(false)

// File dialog
const fileDialog = ref(false)
const fileNodeId = ref<number | null>(null)
const filePath = ref('/')
const files = ref<FileEntry[]>([])
const fileLoading = ref(false)

// ── Data loading ───────────────────────────────────────────────────────────────

async function fetchDashboard() {
  try {
    const resp = await fetch(`${API}/cluster`)
    if (resp.ok) dashboard.value = await resp.json()
  } catch { /* optional feature */ }
}

async function fetchNodes() {
  loading.value = true
  try {
    const resp = await fetch(API)
    if (resp.ok) nodes.value = await resp.json()
  } catch {
    // nodes may not be available
  } finally {
    loading.value = false
  }
}

async function refreshAll() {
  await Promise.all([fetchNodes(), fetchDashboard()])
}

// ── Delete ─────────────────────────────────────────────────────────────────────

async function handleDelete(node: NodeInfo) {
  try {
    await ElMessageBox.confirm(`确定删除节点「${node.name}」？`, '确认', { type: 'warning' })
  } catch { return }
  try {
    const resp = await fetch(`${API}/${node.id}`, { method: 'DELETE' })
    if (!resp.ok) throw new Error('删除失败')
    ElMessage.success('已删除')
    await refreshAll()
  } catch (e: any) {
    ElMessage.error(e.message || '删除失败')
  }
}

// ── Exec ───────────────────────────────────────────────────────────────────────

function openSingleExec(node: NodeInfo) {
  execMode.value = 'single'
  execNodeId.value = node.id
  execCommand.value = ''
  execTimeout.value = 30
  execResults.value = []
  execDialog.value = true
}

function openBatchExec() {
  if (selectedNodes.value.length === 0) {
    ElMessage.warning('请先选择在线节点（勾选卡片）')
    return
  }
  execMode.value = 'batch'
  execNodeId.value = null
  execCommand.value = ''
  execTimeout.value = 30
  execResults.value = []
  execDialog.value = true
}

async function runExec() {
  if (!execCommand.value.trim()) {
    ElMessage.warning('请输入命令')
    return
  }
  execRunning.value = true
  execResults.value = []
  try {
    if (execMode.value === 'single' && execNodeId.value) {
      const resp = await fetch(`${API}/${execNodeId.value}/exec`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          command: execCommand.value,
          timeout_secs: execTimeout.value,
        }),
      })
      if (!resp.ok) { const err = await resp.text(); throw new Error(err) }
      const r = await resp.json()
      execResults.value = [r]
    } else {
      const resp = await fetch(`${API}/batch-exec`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          node_ids: selectedNodes.value,
          command: execCommand.value,
          timeout_secs: execTimeout.value,
        }),
      })
      if (!resp.ok) { const err = await resp.text(); throw new Error(err) }
      execResults.value = await resp.json()
    }
  } catch (e: any) {
    ElMessage.error('执行失败: ' + e.message)
  } finally {
    execRunning.value = false
  }
}

// ── File browser ───────────────────────────────────────────────────────────────

function openFileBrowser(node: NodeInfo) {
  fileNodeId.value = node.id
  filePath.value = '/'
  loadFiles()
  fileDialog.value = true
}

async function loadFiles() {
  if (!fileNodeId.value) return
  fileLoading.value = true
  try {
    const params = new URLSearchParams()
    if (filePath.value) params.set('path', filePath.value)
    const resp = await fetch(`${API}/${fileNodeId.value}/files/list?${params}`)
    if (!resp.ok) throw new Error('加载失败')
    files.value = await resp.json()
  } catch (e: any) {
    ElMessage.error('加载文件列表失败: ' + e.message)
  } finally {
    fileLoading.value = false
  }
}

function navigateTo(file: FileEntry) {
  if (!file.is_dir) return
  const sep = filePath.value.endsWith('/') ? '' : '/'
  filePath.value += sep + file.name
  loadFiles()
}

function goUp() {
  if (filePath.value === '/') return
  const parts = filePath.value.split('/').filter(Boolean)
  parts.pop()
  filePath.value = '/' + parts.join('/')
  loadFiles()
}

function downloadNodeFile(file: FileEntry) {
  if (!fileNodeId.value) return
  const fullPath = filePath.value.endsWith('/')
    ? filePath.value + file.name
    : filePath.value + '/' + file.name
  const params = new URLSearchParams({ path: fullPath })
  window.open(`${API}/${fileNodeId.value}/files/download?${params}`, '_blank')
}

// ── Helpers ────────────────────────────────────────────────────────────────────

const onlineNodes = computed(() => nodes.value.filter(n => n.status === 'online'))

function statusTag(status: string) { return status === 'online' ? 'success' : 'danger' }
function statusText(status: string) { return status === 'online' ? '在线' : '离线' }

function formatDate(s: string) {
  if (!s) return ''
  return s.replace('T', ' ').substring(0, 16)
}

function formatSize(bytes: number): string {
  if (bytes === 0 || bytes == null) return '-'
  const units = ['B', 'KB', 'MB', 'GB']
  let size = bytes, idx = 0
  while (size >= 1024 && idx < units.length - 1) { size /= 1024; idx++ }
  return `${size.toFixed(1)} ${units[idx]}`
}

function toggleSelectNode(id: number) {
  const idx = selectedNodes.value.indexOf(id)
  if (idx >= 0) selectedNodes.value.splice(idx, 1)
  else selectedNodes.value.push(id)
}

onMounted(refreshAll)
</script>

<template>
  <div class="nodes-page" v-loading="loading">
    <div class="page-header">
      <h1>节点管理</h1>
      <div class="header-actions">
        <el-button v-if="selectedNodes.length > 0" type="warning" @click="openBatchExec()">
          批量执行 ({{ selectedNodes.length }})
        </el-button>
        <el-button type="primary" :icon="Refresh" @click="refreshAll">刷新</el-button>
      </div>
    </div>

    <!-- Cluster Summary -->
    <el-row v-if="dashboard" :gutter="16" class="cluster-summary">
      <el-col :span="4">
        <div class="stat-card">
          <div class="stat-val">{{ dashboard.total_nodes }}</div>
          <div class="stat-label">总节点</div>
        </div>
      </el-col>
      <el-col :span="4">
        <div class="stat-card online">
          <div class="stat-val">{{ dashboard.online_nodes }}</div>
          <div class="stat-label">在线</div>
        </div>
      </el-col>
      <el-col :span="4">
        <div class="stat-card offline">
          <div class="stat-val">{{ dashboard.offline_nodes }}</div>
          <div class="stat-label">离线</div>
        </div>
      </el-col>
      <el-col :span="4">
        <div class="stat-card">
          <div class="stat-val">{{ dashboard.avg_cpu.toFixed(1) }}%</div>
          <div class="stat-label">平均 CPU</div>
        </div>
      </el-col>
      <el-col :span="4">
        <div class="stat-card">
          <div class="stat-val">{{ dashboard.avg_memory.toFixed(1) }}%</div>
          <div class="stat-label">平均内存</div>
        </div>
      </el-col>
      <el-col :span="4">
        <div class="stat-card">
          <div class="stat-val">{{ dashboard.avg_load.toFixed(1) }}</div>
          <div class="stat-label">平均负载</div>
        </div>
      </el-col>
    </el-row>

    <!-- Node Cards -->
    <el-row :gutter="20">
      <el-col
        v-for="node in nodes"
        :key="node.id"
        :span="8"
        style="margin-bottom: 20px"
      >
        <el-card
          class="node-card"
          :class="{ selected: selectedNodes.includes(node.id) }"
          @click="node.status === 'online' && toggleSelectNode(node.id)"
        >
          <template #header>
            <div class="node-header">
              <span class="node-name">{{ node.name }}</span>
              <el-tag :type="statusTag(node.status)" size="small">
                {{ statusText(node.status) }}
              </el-tag>
            </div>
          </template>

          <div class="node-body">
            <div class="node-row">
              <span class="label">主机</span>
              <span>{{ node.host }}:{{ node.agent_port }}</span>
            </div>
            <div class="node-row">
              <span class="label">CPU</span>
              <el-progress :percentage="Math.round(node.cpu_usage)" :color="node.cpu_usage > 80 ? '#f56c6c' : '#409eff'" :stroke-width="8" />
            </div>
            <div class="node-row">
              <span class="label">内存</span>
              <el-progress :percentage="Math.round(node.memory_usage_percent)" :color="node.memory_usage_percent > 80 ? '#f56c6c' : '#67c23a'" :stroke-width="8" />
            </div>
            <div class="node-row">
              <span class="label">磁盘</span>
              <el-progress :percentage="Math.round(node.disk_usage_percent)" :color="node.disk_usage_percent > 80 ? '#f56c6c' : '#e6a23c'" :stroke-width="8" />
            </div>
            <div class="node-row">
              <span class="label">负载 (1m)</span>
              <span>{{ node.load_one.toFixed(2) }}</span>
            </div>
            <div class="node-row">
              <span class="label">最后心跳</span>
              <span class="sub-text">{{ formatDate(node.last_heartbeat) }}</span>
            </div>
          </div>

          <div class="node-actions" @click.stop>
            <el-button v-if="node.status === 'online'" size="small" text type="primary" @click="openSingleExec(node)">
              执行命令
            </el-button>
            <el-button v-if="node.status === 'online'" size="small" text type="success" @click="openFileBrowser(node)">
              文件
            </el-button>
            <el-button size="small" text type="danger" @click="handleDelete(node)">删除</el-button>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-empty v-if="nodes.length === 0 && !loading" description="暂无节点">
      <div class="setup-hint">
        <p>在目标服务器上部署 Agent 以添加节点：</p>
        <el-alert type="info" :closable="false" show-icon>
          <template #title>
            <code>PANEL_URL=http://面板IP:8080 AUTH_TOKEN=your-token flamepanel-agent</code>
          </template>
        </el-alert>
      </div>
    </el-empty>

    <!-- Exec Dialog -->
    <el-dialog v-model="execDialog" :title="execMode === 'single' ? '远程执行命令' : `批量执行命令 (${selectedNodes.length} 个节点)`" width="720px">
      <el-form label-width="80px">
        <el-form-item label="命令">
          <el-input v-model="execCommand" type="textarea" :rows="3" placeholder="如: uptime" />
        </el-form-item>
        <el-form-item label="超时(秒)">
          <el-input-number v-model="execTimeout" :min="5" :max="300" />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="runExec" :loading="execRunning">执行</el-button>
        </el-form-item>
      </el-form>

      <div v-if="execResults.length > 0" class="exec-results">
        <div v-for="r in execResults" :key="r.node_id" class="exec-result-item">
          <div class="exec-result-header">
            <span class="exec-node-name">{{ r.node_name || '#' + r.node_id }}</span>
            <span class="exec-meta">
              <el-tag :type="r.exit_code === 0 ? 'success' : 'danger'" size="small">
                exit: {{ r.exit_code }}
              </el-tag>
              <span style="font-size: 12px; color: var(--text-secondary); margin-left: 8px">{{ r.duration_ms }}ms</span>
            </span>
          </div>
          <pre class="exec-output">{{ r.output }}</pre>
        </div>
      </div>
      <template #footer>
        <el-button @click="execDialog = false">关闭</el-button>
      </template>
    </el-dialog>

    <!-- File Browser Dialog -->
    <el-dialog v-model="fileDialog" title="节点文件管理" width="700px">
      <div class="file-nav">
        <el-button size="small" @click="goUp">上一步</el-button>
        <el-input v-model="filePath" size="small" style="margin-left: 8px; flex: 1" readonly />
        <el-button size="small" :icon="Refresh" @click="loadFiles" :loading="fileLoading" style="margin-left: 8px" />
      </div>

      <div class="file-list" v-loading="fileLoading">
        <div v-if="files.length === 0 && !fileLoading" style="text-align: center; padding: 40px; color: var(--text-secondary)">
          目录为空
        </div>
        <div
          v-for="f in files"
          :key="f.name"
          class="file-row"
          :class="{ clickable: f.is_dir }"
          @click="navigateTo(f)"
        >
          <span class="file-icon">{{ f.is_dir ? '📁' : '📄' }}</span>
          <span class="file-name">{{ f.name }}</span>
          <span class="file-size">{{ f.is_dir ? '-' : formatSize(f.size) }}</span>
          <span class="file-actions">
            <el-button v-if="!f.is_dir" size="small" text type="primary" @click.stop="downloadNodeFile(f)">下载</el-button>
          </span>
        </div>
      </div>
      <template #footer>
        <el-button @click="fileDialog = false">关闭</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.nodes-page { padding: 20px; }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }
.page-header h1 { margin: 0; }
.header-actions { display: flex; gap: 8px; }

.cluster-summary { margin-bottom: 24px; }
.stat-card {
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 10px;
  padding: 16px;
  text-align: center;
}
.stat-card.online { border-color: #67c23a; }
.stat-card.offline { border-color: #f56c6c; }
.stat-val { font-size: 24px; font-weight: 700; color: var(--text-primary); }
.stat-label { font-size: 12px; color: var(--text-secondary); margin-top: 4px; }

.node-card { cursor: pointer; transition: border-color 0.2s; }
.node-card.selected { border-color: #409eff; border-width: 2px; }
.node-header { display: flex; justify-content: space-between; align-items: center; }
.node-name { font-weight: 600; font-size: 15px; }
.node-body { display: flex; flex-direction: column; gap: 10px; }
.node-row { display: flex; align-items: center; gap: 12px; }
.node-row .label { width: 72px; font-size: 12px; color: var(--text-secondary); flex-shrink: 0; }
.node-row .el-progress { flex: 1; }
.sub-text { font-size: 12px; color: var(--text-secondary); }
.node-actions { margin-top: 12px; text-align: right; display: flex; gap: 4px; justify-content: flex-end; }
.setup-hint { margin-top: 12px; }
.setup-hint p { color: var(--text-secondary); font-size: 13px; margin-bottom: 8px; }
.setup-hint code { font-size: 12px; word-break: break-all; }

.exec-results { margin-top: 16px; }
.exec-result-item {
  background: var(--bg-hover);
  border-radius: 8px;
  padding: 12px;
  margin-bottom: 8px;
}
.exec-result-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px; }
.exec-node-name { font-weight: 600; }
.exec-meta { display: flex; align-items: center; }
.exec-output {
  background: #1e1e1e;
  color: #d4d4d4;
  padding: 10px;
  border-radius: 6px;
  font-size: 13px;
  max-height: 200px;
  overflow-y: auto;
  white-space: pre-wrap;
  word-break: break-all;
  margin: 0;
}

.file-nav { display: flex; align-items: center; margin-bottom: 12px; }
.file-list { min-height: 200px; }
.file-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 6px;
  transition: background 0.15s;
}
.file-row:hover { background: var(--bg-hover); }
.file-row.clickable { cursor: pointer; }
.file-icon { font-size: 18px; width: 24px; text-align: center; }
.file-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.file-size { font-size: 12px; color: var(--text-secondary); width: 70px; text-align: right; }
.file-actions { width: 60px; text-align: right; }
</style>
