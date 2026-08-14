<template>
  <LayoutContent :title="t('log.title')">
    <template #toolbar>
      <FpTag
        :severity="wsConnected ? 'success' : 'danger'"
        :value="wsConnected ? t('log.wsConnected') : t('log.wsDisconnected')"
        dot
      />
      <span class="text-xs text-muted">{{ logs.length }} {{ t('log.entries') }}</span>
    </template>

    <div class="panel">
      <div class="log-filter">
        <FpSelect
          v-model="levelFilter"
          :options="levelOptions"
          option-label="label"
          option-value="value"
        />
      </div>
      <FpStatePanel
        :loading="wsConnected && !logs.length"
        :error="!wsConnected && !logs.length ? t('log.connectionFailed') : null"
        :empty="!logs.length"
        retryable
        :empty-title="t('common.noData')"
        :empty-desc="t('log.waiting')"
        @retry="reconnect"
      >
        <FpTable
          :rows="filteredLogs"
          :paginator="false"
          :empty-text="t('common.noData')"
          striped-rows
          virtual
          virtual-scroll-height="620px"
        >
          <FpColumn field="id" :header="t('log.id')" style="width: 60px" />
          <FpColumn field="source" :header="t('log.source')" style="width: 120px" />
          <FpColumn :header="t('log.level')" style="width: 90px">
            <template #body="{ data }">
              <FpTag :severity="levelSeverity(data.level)" :value="data.level" />
            </template>
          </FpColumn>
          <FpColumn :header="t('log.message')" style="min-width: 300px">
            <template #body="{ data }">
              <span v-tooltip="data.message" class="log-message">{{ data.message }}</span>
            </template>
          </FpColumn>
          <FpColumn field="created_at" :header="t('log.time')" style="width: 180px">
            <template #body="{ data }">
              <span class="mono">{{ data.created_at }}</span>
            </template>
          </FpColumn>
        </FpTable>
      </FpStatePanel>
    </div>
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'

import LayoutContent from '@/components/ui/LayoutContent.vue'
import FpTable from '@/components/ui/FpTable.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpStatePanel from '@/components/ui/FpStatePanel.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import { useWebSocket } from '@/composables/useWebSocket'
import type { LogEntry } from '@/types'

const { t } = useI18n()
const logs = ref<LogEntry[]>([])
const levelFilter = ref('')
const wsConnected = ref(false)
const loadFailed = ref(false)

const wsLogs = useWebSocket('/ws/logs', {
  onStatus: (connected) => {
    wsConnected.value = connected
    if (connected) loadFailed.value = false
  },
  onMessage: (data) => {
    const msg = data as { type: string; data: LogEntry }
    if (msg.type === 'init') {
      logs.value = msg.data as unknown as LogEntry[]
      loadFailed.value = false
    } else if (msg.type === 'tick') {
      logs.value.unshift(msg.data)
      if (logs.value.length > 500) logs.value.length = 500
    }
  },
})

onMounted(() => wsLogs.connect())

const levelOptions = computed(() => [
  { label: t('log.all'), value: '' },
  { label: t('log.info'), value: 'info' },
  { label: t('log.warn'), value: 'warn' },
  { label: t('log.error'), value: 'error' },
  { label: t('log.critical'), value: 'critical' },
])

const filteredLogs = computed(() =>
  levelFilter.value ? logs.value.filter((l) => l.level === levelFilter.value) : logs.value,
)

function levelSeverity(lv: string): 'success' | 'warning' | 'danger' | 'info' | 'neutral' {
  const map: Record<string, 'success' | 'warning' | 'danger' | 'info' | 'neutral'> = {
    info: 'info',
    warn: 'warning',
    error: 'danger',
    critical: 'danger',
  }
  return map[lv] || 'info'
}

function reconnect() {
  loadFailed.value = false
  wsLogs.reconnectNow()
}
</script>

<style scoped>
.log-filter {
  width: 180px;
  margin-bottom: var(--fp-space-3);
}
.log-message {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
