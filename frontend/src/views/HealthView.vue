<template>
  <div class="view-container">
    <div class="card-header-title">
    </div>
    <el-row :gutter="16">
      <el-col :xs="24" :md="8">
        <el-card shadow="hover">
          <template #header>{{ t('health.backend') }}</template>
          <div class="health-item">
            <span class="dot green" />
            <span>{{ t('health.connected') }}</span>
          </div>
          <div class="health-detail">Status: 200 OK</div>
        </el-card>
      </el-col>
      <el-col :xs="24" :md="8">
        <el-card shadow="hover">
          <template #header>{{ t('health.websocket') }}</template>
          <div class="health-item">
            <span class="dot" :class="wsOk ? 'green' : 'red'" />
            <span>{{ wsOk ? t('health.connected') : t('health.disconnected') }}</span>
          </div>
          <div class="health-detail">Endpoint: /ws/metrics</div>
        </el-card>
      </el-col>
      <el-col :xs="24" :md="8">
        <el-card shadow="hover">
          <template #header>{{ t('health.storage') }}</template>
          <div class="health-item">
            <span class="dot green" />
            <span>{{ t('health.memory') }}</span>
          </div>
          <div class="health-detail">SQLite</div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" class="mt-4">
      <template #header>{{ t('health.routes') }}</template>
      <el-table
        :data="routes"
        border
        stripe
        size="small"
        max-height="400px"
        :empty-text="t('common.noData')"
      >
        <el-table-column :label="t('health.method')" width="100">
          <template #default="{ row }">
            <el-tag
              size="small"
              :type="
                row.method === 'GET'
                  ? 'success'
                  : row.method === 'POST'
                    ? 'warning'
                    : row.method === 'WS'
                      ? 'primary'
                      : 'info'
              "
              >{{ row.method }}</el-tag
            >
          </template>
        </el-table-column>
        <el-table-column :label="t('health.path')" prop="path" />
        <el-table-column :label="t('health.auth')" width="80">
          <template #default="{ row }">
            <el-tag size="small" :type="row.auth ? 'danger' : 'info'">{{
              row.auth ? t('health.required') : t('health.none')
            }}</el-tag>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const wsOk = ref(false)
let ws: WebSocket | null = null

const routes = [
  { method: 'GET', path: '/health', auth: false },
  { method: 'POST', path: '/api/auth/login', auth: false },
  { method: 'POST', path: '/api/auth/change-password', auth: true },
  { method: 'GET|POST', path: '/api/users', auth: true },
  { method: 'DELETE', path: '/api/users/:id', auth: true },
  { method: 'GET|POST', path: '/api/nodes', auth: true },
  { method: 'DELETE', path: '/api/nodes/:id', auth: true },
  { method: 'GET|POST', path: '/api/websites', auth: true },
  { method: 'GET', path: '/api/docker/containers', auth: true },
  { method: 'GET', path: '/api/docker/containers/:id', auth: true },
  { method: 'POST', path: '/api/docker/containers/:id/start|stop|restart|remove', auth: true },
  { method: 'GET', path: '/api/docker/containers/:id/logs|stats', auth: true },
  { method: 'GET|POST', path: '/api/docker/images', auth: true },
  { method: 'POST', path: '/api/docker/images/:id/remove', auth: true },
  { method: 'POST', path: '/api/docker/compose/deploy|up|down', auth: true },
  { method: 'GET|POST', path: '/api/plugins', auth: true },
  { method: 'GET|POST|DELETE', path: '/api/plugins/:id', auth: true },
  { method: 'POST', path: '/api/plugins/:id/reload|enable|disable', auth: true },
  { method: 'POST', path: '/api/plugins/:id/execute/:fn', auth: true },
  { method: 'GET|DELETE', path: '/api/plugins/:id/metrics', auth: true },
  { method: 'GET|POST', path: '/api/plugins/:id/settings', auth: true },
  { method: 'GET', path: '/api/plugins/:id/settings/:key', auth: true },
  { method: 'GET|POST|PUT|DELETE', path: '/api/web-servers', auth: true },
  { method: 'GET|POST|PUT|DELETE', path: '/api/web-servers/:id', auth: true },
  { method: 'POST', path: '/api/web-servers/:id/start|stop|restart|reload|configtest', auth: true },
  { method: 'GET|POST', path: '/api/web-servers/:id/config', auth: true },
  { method: 'GET|PUT', path: '/api/settings', auth: true },
  { method: 'GET', path: '/api/settings/:key', auth: true },
  { method: 'GET', path: '/api/operation-logs', auth: true },
  { method: 'GET', path: '/api/logs', auth: true },
  { method: 'GET|POST|DELETE', path: '/api/databases', auth: true },
  { method: 'GET|POST|DELETE', path: '/api/databases/:id', auth: true },
  { method: 'POST', path: '/api/databases/mysql/install|redis/install', auth: true },
  { method: 'POST', path: '/api/databases/:id/start|stop|restart|uninstall', auth: true },
  { method: 'GET', path: '/api/databases/:id/status', auth: true },
  { method: 'GET|POST|DELETE', path: '/api/databases/:id/databases', auth: true },
  { method: 'DELETE', path: '/api/databases/:id/databases/:db_name', auth: true },
  { method: 'POST|DELETE', path: '/api/databases/:id/users', auth: true },
  { method: 'GET|POST|DELETE', path: '/api/files', auth: true },
  { method: 'GET', path: '/api/files/read|download', auth: true },
  {
    method: 'POST',
    path: '/api/files/write|create-file|create-dir|rename|chmod|upload',
    auth: true,
  },
  { method: 'DELETE', path: '/api/files/delete', auth: true },
  { method: 'GET|POST|PUT|DELETE', path: '/api/firewall/rules', auth: true },
  { method: 'GET|PUT|DELETE', path: '/api/firewall/rules/:id', auth: true },
  { method: 'POST', path: '/api/firewall/rules/:id/toggle', auth: true },
  { method: 'POST', path: '/api/firewall/apply|enable|disable|reorder', auth: true },
  { method: 'GET', path: '/api/firewall/status', auth: true },
  { method: 'WS', path: '/ws/metrics', auth: false },
  { method: 'WS', path: '/ws/logs', auth: false },
  { method: 'WS', path: '/ws/terminal', auth: true },
]

onMounted(() => {
  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
  ws = new WebSocket(`${protocol}//${location.host}/ws/metrics`)
  ws.onopen = () => {
    wsOk.value = true
  }
  ws.onclose = () => {
    wsOk.value = false
  }
  ws.onerror = () => {
    wsOk.value = false
  }
})

onUnmounted(() => ws?.close())
</script>

<style scoped>
.health-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 15px;
  font-weight: 600;
}
.health-detail {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  margin-top: 6px;
}
.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  display: inline-block;
}
.dot.green {
  background: #67c23a;
}
.dot.red {
  background: #f56c6c;
}
</style>
