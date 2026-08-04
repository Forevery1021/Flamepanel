<template>
  <div class="view-container">
    <div class="card-header-title">
    </div>
    <el-row :gutter="16">
      <el-col :xs="24" :md="8">
        <el-card shadow="hover">
          <template #header>{{ t('health.database') }}</template>
          <div class="health-item">
            <span class="dot" :class="dbOk ? 'green' : 'red'" />
            <span>{{ dbOk ? t('health.connected') : t('health.disconnected') }}</span>
          </div>
          <div class="health-detail">{{ dbDetail }}</div>
        </el-card>
      </el-col>
      <el-col :xs="24" :md="8">
        <el-card shadow="hover">
          <template #header>{{ t('health.docker') }}</template>
          <div class="health-item">
            <span class="dot" :class="dockerOk ? 'green' : 'red'" />
            <span>{{ dockerOk ? t('health.connected') : t('health.degraded') }}</span>
          </div>
          <div class="health-detail">{{ dockerDetail }}</div>
        </el-card>
      </el-col>
      <el-col :xs="24" :md="8">
        <el-card shadow="hover">
          <template #header>{{ t('health.disk') }}</template>
          <div class="health-item">
            <span class="dot" :class="diskOk ? 'green' : 'red'" />
            <span>{{ diskOk ? t('health.connected') : t('health.degraded') }}</span>
          </div>
          <div class="health-detail">{{ diskDetail }}</div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" class="mt-4">
      <template #header>
        <span>{{ t('health.panel') }}</span>
        <el-tag :type="panelStatus === 'ok' ? 'success' : 'warning'" size="small" class="ml-2">
          {{ panelStatus }}
        </el-tag>
      </template>
      <div class="info-rows">
        <div class="info-row"><span>{{ t('health.version') }}</span><span>{{ health?.version || '—' }}</span></div>
        <div class="info-row"><span>{{ t('health.uptime') }}</span><span>{{ uptimeText }}</span></div>
        <div class="info-row"><span>{{ t('health.websocket') }}</span><span>{{ wsOk ? t('health.connected') : t('health.disconnected') }}</span></div>
      </div>
    </el-card>

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
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { connectWithRetry } from '@/utils/ws'
import { fetchHealthDetail } from '@/api/health'
import type { HealthDetail } from '@/types'

const { t } = useI18n()
const wsOk = ref(false)
let wsConn: ReturnType<typeof connectWithRetry> | null = null

// ── 真实 /api/health 数据 ──
const health = ref<HealthDetail | null>(null)
const dbOk = ref(false)
const dockerOk = ref(false)
const diskOk = ref(false)
const dbDetail = ref('—')
const dockerDetail = ref('—')
const diskDetail = ref('—')
const panelStatus = ref('unknown')

const uptimeText = computed(() => {
  const s = health.value?.uptime_secs ?? 0
  const h = Math.floor(s / 3600)
  const m = Math.floor((s % 3600) / 60)
  return `${h}h ${m}m`
})

async function refreshHealth() {
  try {
    const res = await fetchHealthDetail()
    health.value = res.data
    panelStatus.value = res.data.status
    const c = res.data.checks
    dbOk.value = c.database.status === 'ok'
    dockerOk.value = c.docker.status === 'ok'
    diskOk.value = c.disk.status === 'ok'
    dbDetail.value = c.database.detail || c.database.status
    dockerDetail.value = c.docker.detail || c.docker.status
    diskDetail.value = c.disk.detail || c.disk.status
  } catch {
    panelStatus.value = 'error'
    dbOk.value = false
    dockerOk.value = false
    diskOk.value = false
  }
}

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
  refreshHealth()
  wsConn = connectWithRetry('/ws/metrics', {
    onStatus: (connected) => {
      wsOk.value = connected
    },
    onMessage: () => {},
  })
})

onUnmounted(() => wsConn?.close())
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
.info-rows {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.info-row {
  display: flex;
  justify-content: space-between;
  font-size: 13px;
  color: var(--text-secondary);
}
</style>
