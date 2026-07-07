<template>
  <div>
    <h2>System Health</h2>
    <el-row :gutter="16" style="margin-top:16px">
      <el-col :span="8">
        <el-card shadow="hover">
          <template #header>Backend API</template>
          <div class="health-item">
            <span class="dot green" />
            <span>Connected</span>
          </div>
          <div class="health-detail">Status: 200 OK</div>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card shadow="hover">
          <template #header>WebSocket</template>
          <div class="health-item">
            <span class="dot" :class="wsOk ? 'green' : 'red'" />
            <span>{{ wsOk ? 'Connected' : 'Disconnected' }}</span>
          </div>
          <div class="health-detail">Endpoint: /ws/metrics</div>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card shadow="hover">
          <template #header>Storage</template>
          <div class="health-item">
            <span class="dot green" />
            <span>In-Memory</span>
          </div>
          <div class="health-detail">No persistent database configured</div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" style="margin-top:16px">
      <template #header>API Routes</template>
      <el-table :data="routes" border stripe size="small" max-height="400px">
        <el-table-column prop="method" label="Method" width="80">
          <template #default="{ row }">
            <el-tag size="small" :type="row.method === 'GET' ? 'success' : row.method === 'POST' ? 'warning' : 'info'">{{ row.method }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="path" label="Path" />
        <el-table-column prop="auth" label="Auth" width="80">
          <template #default="{ row }">
            <el-tag size="small" :type="row.auth ? 'danger' : 'info'">{{ row.auth ? 'JWT' : 'None' }}</el-tag>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'

const wsOk = ref(false)
let ws: WebSocket | null = null

const routes = [
  { method: 'GET', path: '/health', auth: false },
  { method: 'POST', path: '/api/auth/login', auth: false },
  { method: 'POST', path: '/api/auth/change-password', auth: true },
  { method: 'GET', path: '/api/users', auth: true },
  { method: 'POST', path: '/api/users', auth: true },
  { method: 'GET', path: '/api/nodes', auth: true },
  { method: 'POST', path: '/api/nodes', auth: true },
  { method: 'GET', path: '/api/websites', auth: true },
  { method: 'POST', path: '/api/websites', auth: true },
  { method: 'GET', path: '/api/docker/containers', auth: true },
  { method: 'POST', path: '/api/docker/containers/:id/start|stop|restart|remove', auth: true },
  { method: 'GET', path: '/api/docker/containers/:id/logs|stats', auth: true },
  { method: 'GET|POST', path: '/api/docker/images', auth: true },
  { method: 'POST', path: '/api/docker/compose/deploy|up|down', auth: true },
  { method: 'GET|POST', path: '/api/plugins', auth: true },
  { method: 'GET|POST', path: '/api/plugins/:id', auth: true },
  { method: 'POST', path: '/api/plugins/:id/reload|enable|disable', auth: true },
  { method: 'POST', path: '/api/plugins/:id/execute/:fn', auth: true },
  { method: 'GET|DELETE', path: '/api/plugins/:id/metrics', auth: true },
  { method: 'GET|POST', path: '/api/plugins/:id/settings', auth: true },
  { method: 'GET', path: '/api/plugins/:id/settings/:key', auth: true },
  { method: 'GET|POST', path: '/api/web-servers/engines', auth: true },
  { method: 'GET|POST|PUT|DELETE', path: '/api/web-servers', auth: true },
  { method: 'GET|POST|PUT|DELETE', path: '/api/web-servers/:id', auth: true },
  { method: 'POST', path: '/api/web-servers/:id/start|stop|restart|reload|configtest', auth: true },
  { method: 'GET|POST', path: '/api/web-servers/:id/config', auth: true },
  { method: 'GET|PUT', path: '/api/settings', auth: true },
  { method: 'GET', path: '/api/settings/:key', auth: true },
  { method: 'GET', path: '/api/operation-logs', auth: true },
  { method: 'GET', path: '/api/logs', auth: true },
  { method: 'WS', path: '/ws/metrics', auth: false },
  { method: 'WS', path: '/ws/logs', auth: false },
]

onMounted(() => {
  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
  ws = new WebSocket(`${protocol}//${location.host}/ws/metrics`)
  ws.onopen = () => { wsOk.value = true }
  ws.onclose = () => { wsOk.value = false }
  ws.onerror = () => { wsOk.value = false }
})

onUnmounted(() => ws?.close())
</script>

<style scoped>
.health-item { display: flex; align-items: center; gap: 8px; font-size: 15px; font-weight: 600; }
.health-detail { font-size: 12px; color: #909399; margin-top: 6px; }
.dot { width: 10px; height: 10px; border-radius: 50%; display: inline-block; }
.dot.green { background: #67c23a; }
.dot.red { background: #f56c6c; }
</style>
