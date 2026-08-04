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
      <FpTable
        :rows="filteredLogs"
        :paginator="false"
        :empty-text="t('common.noData')"
        striped-rows
        scrollable
        scroll-height="620px"
      >
        <Column field="id" :header="t('log.id')" style="width: 60px" />
        <Column field="source" :header="t('log.source')" style="width: 120px" />
        <Column :header="t('log.level')" style="width: 90px">
          <template #body="{ data }">
            <FpTag :severity="levelSeverity(data.level)" :value="data.level" />
          </template>
        </Column>
        <Column :header="t('log.message')" style="min-width: 300px">
          <template #body="{ data }">
            <span v-tooltip="data.message" class="log-message">{{ data.message }}</span>
          </template>
        </Column>
        <Column field="created_at" :header="t('log.time')" style="width: 180px">
          <template #body="{ data }">
            <span class="mono">{{ data.created_at }}</span>
          </template>
        </Column>
      </FpTable>
    </div>
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import Column from 'openvue/column'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import FpTable from '@/components/ui/FpTable.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpTag from '@/components/ui/FpTag.vue'
import { connectWithRetry } from '@/utils/ws'
import type { LogEntry } from '@/types'

const { t } = useI18n()
const logs = ref<LogEntry[]>([])
const levelFilter = ref('')
const wsConnected = ref(false)
let wsConn: ReturnType<typeof connectWithRetry> | null = null

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

onMounted(() => {
  wsConn = connectWithRetry('/ws/logs', {
    onStatus: (connected) => {
      wsConnected.value = connected
    },
    onMessage: (data) => {
      const msg = data as { type: string; data: LogEntry }
      if (msg.type === 'init') {
        logs.value = msg.data as unknown as LogEntry[]
      } else if (msg.type === 'tick') {
        logs.value.unshift(msg.data)
        if (logs.value.length > 500) logs.value.length = 500
      }
    },
  })
})

onUnmounted(() => wsConn?.close())
</script>

<style scoped>
.panel {
  padding: var(--fp-space-4);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
}
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
