<template>
  <div>
    <div style="display:flex;justify-content:space-between;align-items:center">
      <h2>System Logs</h2>
      <div style="display:flex;gap:8px;align-items:center">
        <el-select v-model="levelFilter" size="small" style="width:120px" clearable placeholder="All levels">
          <el-option label="All" value="" />
          <el-option label="Info" value="info" />
          <el-option label="Warn" value="warn" />
          <el-option label="Error" value="error" />
          <el-option label="Critical" value="critical" />
        </el-select>
        <el-tag size="small" type="info">{{ filteredLogs.length }} entries</el-tag>
        <el-tag size="small" :type="wsConnected ? 'success' : 'danger'">
          WS {{ wsConnected ? 'live' : 'off' }}
        </el-tag>
      </div>
    </div>
    <el-table
      :data="filteredLogs" border stripe v-loading="loading" style="margin-top:16px"
      max-height="620px" ref="tableRef"
      @scroll="handleScroll"
    >
      <el-table-column prop="id" label="ID" width="60" />
      <el-table-column prop="source" label="Source" width="100" />
      <el-table-column prop="level" label="Level" width="80">
        <template #default="{ row }">
          <el-tag size="small" :type="levelType(row.level)" effect="dark">{{ row.level }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="message" label="Message" min-width="350">
        <template #default="{ row }">
          <div class="log-msg">{{ row.message }}</div>
        </template>
      </el-table-column>
      <el-table-column prop="created_at" label="Time" width="180" />
    </el-table>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { listSystemLogs } from '@/api/logs'
import { ElMessage } from 'element-plus'
import type { LogEntry } from '@/types'

const logs = ref<LogEntry[]>([])
const loading = ref(false)
const levelFilter = ref('')
const wsConnected = ref(false)
const tableRef = ref()
let ws: WebSocket | null = null
let autoScroll = true

const filteredLogs = computed(() => {
  if (!levelFilter.value) return logs.value
  return logs.value.filter(l => l.level === levelFilter.value)
})

function levelType(level: string) {
  if (level === 'error' || level === 'critical') return 'danger'
  if (level === 'warn') return 'warning'
  return 'info'
}

function handleScroll(e: Event) {
  const el = e.target as HTMLElement
  autoScroll = el.scrollTop + el.clientHeight >= el.scrollHeight - 50
}

async function fetch() {
  loading.value = true
  try { logs.value = (await listSystemLogs()).data } catch { ElMessage.error('获取系统日志失败') } finally { loading.value = false }
}

onMounted(() => {
  fetch()
  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
  ws = new WebSocket(`${protocol}//${location.host}/ws/logs`)
  ws.onopen = () => { wsConnected.value = true }
  ws.onclose = () => { wsConnected.value = false }
  ws.onmessage = (ev: MessageEvent) => {
    const msg = JSON.parse(ev.data)
    if (msg.type === 'init') {
      logs.value = msg.data
    } else if (msg.type === 'tick') {
      logs.value.unshift(msg.data)
      if (logs.value.length > 500) logs.value = logs.value.slice(0, 500)
    }
  }
})

onUnmounted(() => ws?.close())
</script>

<style scoped>
.log-msg {
  max-width: 500px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
</style>
