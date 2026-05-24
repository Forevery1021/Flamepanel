<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Refresh, VideoPlay, VideoPause } from '@element-plus/icons-vue'

interface PluginInfo {
  name: string
  version: string
  author: string
  description: string | null
  state: string
  api_prefix: string
}

const plugins = ref<PluginInfo[]>([])
const loading = ref(false)

async function loadPlugins() {
  loading.value = true
  try {
    const resp = await fetch('/api/plugins')
    if (!resp.ok) throw new Error('加载失败')
    plugins.value = await resp.json()
  } catch (e: any) {
    ElMessage.error('加载插件列表失败: ' + e.message)
  } finally {
    loading.value = false
  }
}

async function startPlugin(name: string) {
  try {
    const resp = await fetch(`/api/plugins/${name}/start`)
    if (!resp.ok) { const err = await resp.text(); throw new Error(err) }
    ElMessage.success(`插件 "${name}" 已启动`)
    loadPlugins()
  } catch (e: any) {
    ElMessage.error('启动失败: ' + e.message)
  }
}

async function stopPlugin(name: string) {
  try {
    const resp = await fetch(`/api/plugins/${name}/stop`)
    if (!resp.ok) { const err = await resp.text(); throw new Error(err) }
    ElMessage.success(`插件 "${name}" 已停止`)
    loadPlugins()
  } catch (e: any) {
    ElMessage.error('停止失败: ' + e.message)
  }
}

function stateTag(state: string) {
  switch (state) {
    case 'Loaded': return 'info'
    case 'Running': return 'success'
    case 'Stopped': return 'warning'
    case 'Error': return 'danger'
    default: return 'info'
  }
}

function stateText(state: string) {
  switch (state) {
    case 'Loaded': return '已加载'
    case 'Running': return '运行中'
    case 'Stopped': return '已停止'
    case 'Error': return '错误'
    default: return state
  }
}

onMounted(loadPlugins)
</script>

<template>
  <div class="plugins-view">
    <div class="page-header">
      <h2>插件与扩展</h2>
      <el-button type="primary" :icon="Refresh" @click="loadPlugins" :loading="loading">刷新</el-button>
    </div>

    <div class="plugin-grid">
      <div v-for="p in plugins" :key="p.name" class="plugin-card">
        <div class="plugin-card-header">
          <div class="plugin-name-row">
            <span class="plugin-name">{{ p.name }}</span>
            <el-tag :type="stateTag(p.state)" size="small">{{ stateText(p.state) }}</el-tag>
          </div>
          <span class="plugin-version">v{{ p.version }}</span>
        </div>

        <div class="plugin-body">
          <p v-if="p.description" class="plugin-desc">{{ p.description }}</p>
          <div class="plugin-meta">
            <span>作者: {{ p.author }}</span>
            <span>API: /{{ p.api_prefix }}</span>
          </div>
        </div>

        <div class="plugin-actions">
          <el-button
            v-if="p.state !== 'Running'"
            size="small"
            type="primary"
            :icon="VideoPlay"
            @click="startPlugin(p.name)"
          >
            启动
          </el-button>
          <el-button
            v-if="p.state === 'Running'"
            size="small"
            type="warning"
            :icon="VideoPause"
            @click="stopPlugin(p.name)"
          >
            停止
          </el-button>
        </div>
      </div>
    </div>

    <el-empty v-if="!loading && plugins.length === 0" description="暂无插件">
      <div class="setup-hint">
        <p>将插件放入 <code>plugins/</code> 目录即可自动发现。</p>
        <p>每个插件需要一个 <code>plugin.toml</code> 清单文件：</p>
        <el-alert type="info" :closable="false" show-icon style="text-align: left">
          <template #title>
            <pre style="margin: 0; font-size: 12px">name = "my-plugin"
version = "1.0.0"
author = "Author"
description = "插件描述"
entry = "run.sh"
api_prefix = "my-plugin"
permissions = ["read"]</pre>
          </template>
        </el-alert>
      </div>
    </el-empty>

    <el-divider />

    <!-- Built-in MCP Tools -->
    <div class="section">
      <h3>内置 MCP / Skills 工具</h3>
      <p class="section-desc">这些是 Flamepanel 内置的 MCP 工具，可供 AI 助手调用：</p>
      <div class="tool-grid">
        <div class="tool-item">
          <span class="tool-name">system_info</span>
          <span class="tool-desc">获取系统信息 (CPU/内存/磁盘)</span>
        </div>
        <div class="tool-item">
          <span class="tool-name">list_files</span>
          <span class="tool-desc">列出目录文件</span>
        </div>
        <div class="tool-item">
          <span class="tool-name">execute_command</span>
          <span class="tool-desc">执行 Shell 命令</span>
        </div>
        <div class="tool-item">
          <span class="tool-name">docker_list</span>
          <span class="tool-desc">列出 Docker 容器</span>
        </div>
        <div class="tool-item">
          <span class="tool-name">get_metrics</span>
          <span class="tool-desc">获取实时监控指标</span>
        </div>
        <div class="tool-item">
          <span class="tool-name">system_cleanup</span>
          <span class="tool-desc">扫描并清理系统缓存</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.plugins-view {
  padding: 24px;
  height: 100%;
  overflow-y: auto;
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

.plugin-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 16px;
  margin-bottom: 24px;
}

.plugin-card {
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 10px;
  padding: 18px;
}

.plugin-card-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 10px;
}

.plugin-name-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.plugin-name {
  font-weight: 600;
  font-size: 16px;
  color: var(--text-primary);
}

.plugin-version {
  font-size: 12px;
  color: var(--text-secondary);
  font-family: monospace;
}

.plugin-body {
  margin-bottom: 12px;
}

.plugin-desc {
  margin: 0 0 8px 0;
  font-size: 13px;
  color: var(--text-secondary);
}

.plugin-meta {
  display: flex;
  gap: 16px;
  font-size: 12px;
  color: var(--text-secondary);
}

.plugin-actions {
  display: flex;
  gap: 8px;
}

.setup-hint {
  margin-top: 12px;
}

.setup-hint p {
  color: var(--text-secondary);
  font-size: 13px;
  margin-bottom: 8px;
}

.setup-hint pre {
  white-space: pre-wrap;
}

.section h3 {
  margin: 0 0 8px 0;
  font-size: 16px;
  color: var(--text-primary);
}

.section-desc {
  font-size: 13px;
  color: var(--text-secondary);
  margin-bottom: 12px;
}

.tool-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 8px;
}

.tool-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 8px;
}

.tool-name {
  font-weight: 600;
  font-family: monospace;
  color: #409eff;
  font-size: 13px;
  white-space: nowrap;
}

.tool-desc {
  font-size: 13px;
  color: var(--text-secondary);
}
</style>
