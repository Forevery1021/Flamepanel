<template>
  <LayoutContent :title="t('health.title')" reload @reload="refreshHealth">
    <div class="health-grid">
      <div v-for="h in healthCards" :key="h.key" class="panel health-card">
        <div class="health-item">
          <span class="dot" :class="h.ok ? 'green' : 'red'" />
          <span>{{ h.label }}</span>
          <FpTag :severity="h.ok ? 'success' : 'danger'" :value="h.statusText" />
        </div>
        <div class="health-detail">{{ h.detail }}</div>
      </div>
    </div>

    <div class="panel">
      <div class="panel-header">
        <span class="panel-title">{{ t('health.panel') }}</span>
        <FpTag :severity="panelStatus === 'ok' ? 'success' : 'warning'" :value="panelStatus" />
      </div>
      <div class="info-rows">
        <div class="info-row">
          <span>{{ t('health.version') }}</span>
          <span class="mono">{{ health?.version || '—' }}</span>
        </div>
        <div class="info-row">
          <span>{{ t('health.uptime') }}</span>
          <span class="mono">{{ uptimeText }}</span>
        </div>
        <div class="info-row">
          <span>{{ t('health.websocket') }}</span>
          <span class="mono">{{ wsOk ? t('health.connected') : t('health.disconnected') }}</span>
        </div>
      </div>
    </div>

    <div class="panel">
      <div class="panel-header">
        <span class="panel-title">{{ t('health.routes') }}</span>
      </div>
      <FpTable :rows="routes" :paginator="false" size="small">
        <FpColumn :header="t('health.method')" style="width: 130px">
          <template #body="{ data }">
            <FpTag :severity="methodSeverity(data.method)" :value="data.method" />
          </template>
        </FpColumn>
        <FpColumn :header="t('health.path')" field="path">
          <template #body="{ data }">
            <span class="mono path-text">{{ data.path }}</span>
          </template>
        </FpColumn>
        <FpColumn :header="t('health.auth')" style="width: 90px">
          <template #body="{ data }">
            <FpTag :severity="data.auth ? 'danger' : 'neutral'" :value="data.auth ? t('health.required') : t('health.none')" />
          </template>
        </FpColumn>
      </FpTable>
    </div>
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'

import { fetchHealthDetail } from '@/api/health'
import FpTag from '@/components/ui/FpTag.vue'
import FpTable from '@/components/ui/FpTable.vue'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import { useWebSocket } from '@/composables/useWebSocket'
import type { HealthDetail } from '@/api/generated'

const { t } = useI18n()
const wsOk = ref(false)

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

const healthCards = computed(() => [
  {
    key: 'db',
    label: t('health.database'),
    ok: dbOk.value,
    statusText: dbOk.value ? t('health.connected') : t('health.disconnected'),
    detail: dbDetail.value,
  },
  {
    key: 'docker',
    label: t('health.docker'),
    ok: dockerOk.value,
    statusText: dockerOk.value ? t('health.connected') : t('health.degraded'),
    detail: dockerDetail.value,
  },
  {
    key: 'disk',
    label: t('health.disk'),
    ok: diskOk.value,
    statusText: diskOk.value ? t('health.connected') : t('health.degraded'),
    detail: diskDetail.value,
  },
])

function methodSeverity(method: string) {
  if (method.includes('GET')) return 'success'
  if (method.includes('POST')) return 'warning'
  if (method.includes('WS')) return 'info'
  return 'neutral'
}

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

const wsMetrics = useWebSocket('/ws/metrics', {
  onStatus: (connected) => {
    wsOk.value = connected
  },
  onMessage: () => {},
})

onMounted(() => {
  refreshHealth()
  wsMetrics.connect()
})
</script>

<style scoped>
.health-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: var(--fp-space-4);
}
.health-card {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
}
.health-item {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  font-size: 15px;
  font-weight: 600;
}
.health-item .dot {
  flex-shrink: 0;
}
.health-detail {
  font-size: 12px;
  color: var(--fp-text-secondary);
  word-break: break-all;
}
.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  display: inline-block;
}
.dot.green {
  background: var(--fp-success);
  box-shadow: 0 0 0 3px var(--fp-success-soft);
}
.dot.red {
  background: var(--fp-danger);
  box-shadow: 0 0 0 3px var(--fp-danger-soft);
}
.info-rows {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-2);
}
.info-row {
  display: flex;
  justify-content: space-between;
  font-size: 13px;
  color: var(--fp-text-secondary);
}
.path-text {
  font-size: 12.5px;
}
@media (max-width: 768px) {
  .health-grid {
    grid-template-columns: 1fr;
  }
}
</style>
