<template>
  <div class="view-container">
    <div class="card-header-title">
      <div class="row-gap">
        <el-tag :type="wsConnected ? 'success' : 'danger'" size="small">
          {{ wsConnected ? t('log.wsConnected') : t('log.wsDisconnected') }}
        </el-tag>
        <span class="text-xs text-muted">{{ logs.length }} {{ t('log.entries') }}</span>
      </div>
    </div>

    <el-card shadow="hover">
      <div class="mb-3">
        <el-select v-model="levelFilter" class="w-150">
          <el-option :label="t('log.all')" value="" />
          <el-option :label="t('log.info')" value="info" />
          <el-option :label="t('log.warn')" value="warn" />
          <el-option :label="t('log.error')" value="error" />
          <el-option :label="t('log.critical')" value="critical" />
        </el-select>
      </div>
      <el-table
        ref="tableRef"
        :data="filteredLogs"
        border
        stripe
        max-height="620px"
        :empty-text="t('common.noData')"
        @scroll="handleScroll"
      >
        <el-table-column prop="id" :label="t('log.id')" width="60" />
        <el-table-column prop="source" :label="t('log.source')" width="120" />
        <el-table-column :label="t('log.level')" width="90">
          <template #default="{ row }">
            <el-tag size="small" :type="levelType(row.level)">{{ row.level }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="message"
          :label="t('log.message')"
          min-width="300"
          show-overflow-tooltip
        />
        <el-table-column prop="created_at" :label="t('log.time')" width="180" />
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import type { LogEntry } from '@/types'

const { t } = useI18n()
const logs = ref<LogEntry[]>([])
const levelFilter = ref('')
const wsConnected = ref(false)
const tableRef = ref<HTMLElement>()
let ws: WebSocket | null = null

const filteredLogs = computed(() =>
  levelFilter.value ? logs.value.filter((l) => l.level === levelFilter.value) : logs.value,
)

function levelType(lv: string): 'info' | 'primary' | 'success' | 'warning' | 'danger' {
  const map: Record<string, 'info' | 'primary' | 'success' | 'warning' | 'danger'> = {
    info: 'info',
    warn: 'warning',
    error: 'danger',
    critical: 'danger',
  }
  return map[lv] || 'info'
}

function handleScroll(e: Event) {
  const el = e.target as HTMLElement
  if (el) {
    /* scroll tracking available if needed */
  }
}

onMounted(() => {
  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
  ws = new WebSocket(`${protocol}//${location.host}/ws/logs`)
  ws.onopen = () => {
    wsConnected.value = true
  }
  ws.onclose = () => {
    wsConnected.value = false
  }
  ws.onmessage = (ev: MessageEvent) => {
    const msg = JSON.parse(ev.data)
    if (msg.type === 'init') {
      logs.value = msg.data
    } else if (msg.type === 'tick') {
      logs.value.unshift(msg.data)
      if (logs.value.length > 500) logs.value.length = 500
    }
  }
})

onUnmounted(() => ws?.close())
</script>
