<template>
  <div class="view-container">
    <el-row :gutter="16">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat">
            <div class="stat-icon icon-cpu"><el-icon :size="26"><Cpu /></el-icon></div>
            <div class="stat-body">
              <div class="stat-label">{{ t('dashboard.cpu') }}</div>
              <div class="stat-value">{{ snap.cpu_usage.toFixed(1) }}%</div>
              <el-progress
                :percentage="Math.round(snap.cpu_usage)"
                :color="cpuColor"
                :stroke-width="6"
              />
              <div class="stat-detail">{{ snap.cpu_cores }} cores</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat">
            <div class="stat-icon icon-mem"><el-icon :size="26"><Memo /></el-icon></div>
            <div class="stat-body">
              <div class="stat-label">{{ t('dashboard.memory') }}</div>
              <div class="stat-value">{{ snap.memory_usage_percent.toFixed(1) }}%</div>
              <el-progress
                :percentage="Math.round(snap.memory_usage_percent)"
                :color="memColor"
                :stroke-width="6"
              />
              <div class="stat-detail">
                {{ (snap.memory_used_mb / 1024).toFixed(1) }} /
                {{ (snap.memory_total_mb / 1024).toFixed(1) }} GB
              </div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat">
            <div class="stat-icon icon-disk"><el-icon :size="26"><Coin /></el-icon></div>
            <div class="stat-body">
              <div class="stat-label">{{ t('dashboard.disk') }}</div>
              <div class="stat-value">{{ snap.disk_usage_percent.toFixed(1) }}%</div>
              <el-progress
                :percentage="Math.round(snap.disk_usage_percent)"
                :color="diskColor"
                :stroke-width="6"
              />
              <div class="stat-detail">
                {{ snap.disk_used_gb.toFixed(1) }} / {{ snap.disk_total_gb.toFixed(1) }} GB
              </div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat">
            <div class="stat-icon icon-load"><el-icon :size="26"><TrendCharts /></el-icon></div>
            <div class="stat-body">
              <div class="stat-label">{{ t('dashboard.load') }}</div>
              <div class="stat-value">{{ snap.load_one.toFixed(2) }}</div>
              <div class="stat-detail">
                1m: {{ snap.load_one.toFixed(2) }} | 5m: {{ snap.load_five.toFixed(2) }} | 15m:
                {{ snap.load_fifteen.toFixed(2) }}
              </div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="16" class="chart-row">
      <el-col :xs="24" :lg="16">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>{{ t('dashboard.trend') }}</span>
            </div>
          </template>
          <div ref="chartRef" class="chart-box" />
        </el-card>
      </el-col>
      <el-col :xs="24" :lg="8">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>{{ t('dashboard.connection') }}</span>
              <el-tag
                :type="wsConnected ? 'success' : 'danger'"
                size="small"
                effect="light"
              >
                {{ wsConnected ? t('dashboard.wsConnected') : t('dashboard.wsDisconnected') }}
              </el-tag>
            </div>
          </template>
          <div class="info-row">
            <span>{{ t('dashboard.dataPoints') }}</span
            ><span>{{ history.length }}</span>
          </div>
          <div class="info-row">
            <span>{{ t('dashboard.lastUpdate') }}</span
            ><span>{{ lastUpdate || '—' }}</span>
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onUnmounted, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { init, use } from 'echarts/core'
import { LineChart } from 'echarts/charts'
import { GridComponent, TooltipComponent, LegendComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import { Cpu, Memo, Coin, TrendCharts } from '@element-plus/icons-vue'
import type { MetricsSnapshot } from '@/types'
import type { ECharts } from 'echarts/core'

use([LineChart, GridComponent, TooltipComponent, LegendComponent, CanvasRenderer])

const { t } = useI18n()

const snap = reactive<MetricsSnapshot>({
  timestamp: 0,
  cpu_usage: 0,
  cpu_cores: 0,
  memory_usage_percent: 0,
  memory_total_mb: 0,
  memory_used_mb: 0,
  disk_usage_percent: 0,
  disk_total_gb: 0,
  disk_used_gb: 0,
  load_one: 0,
  load_five: 0,
  load_fifteen: 0,
})

const history = ref<MetricsSnapshot[]>([])
const chartRef = ref<HTMLElement>()
const wsConnected = ref(false)
const lastUpdate = ref('')
let chart: ECharts | null = null
let ws: WebSocket | null = null

function cpuColor(p: number) {
  return p > 80 ? '#f56c6c' : p > 50 ? '#e6a23c' : '#67c23a'
}
const memColor = cpuColor
const diskColor = cpuColor

function cssVar(name: string) {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || undefined
}

function chartPalette() {
  return {
    axisLabel: cssVar('--text-secondary') || '#909399',
    gridLine: cssVar('--border-color') || '#e5e7eb',
  }
}

function initChart() {
  if (!chartRef.value) return
  chart = init(chartRef.value)
  const { axisLabel, gridLine } = chartPalette()
  chart.setOption({
    tooltip: { trigger: 'axis' },
    legend: { data: ['CPU %', 'Memory %', 'Disk %'], bottom: 0, textStyle: { color: axisLabel } },
    grid: { left: 50, right: 20, top: 20, bottom: 40 },
    xAxis: {
      type: 'time',
      axisLabel: { fontSize: 11, color: axisLabel },
      splitLine: { lineStyle: { color: gridLine } },
    },
    yAxis: {
      type: 'value',
      max: 100,
      axisLabel: { fontSize: 11, color: axisLabel },
      splitLine: { lineStyle: { color: gridLine } },
    },
    series: [
      {
        name: 'CPU %',
        type: 'line',
        smooth: true,
        showSymbol: false,
        lineStyle: { width: 2 },
        data: [],
      },
      {
        name: 'Memory %',
        type: 'line',
        smooth: true,
        showSymbol: false,
        lineStyle: { width: 2 },
        data: [],
      },
      {
        name: 'Disk %',
        type: 'line',
        smooth: true,
        showSymbol: false,
        lineStyle: { width: 2 },
        data: [],
      },
    ],
  })
  const observer = new MutationObserver(() => {
    const { axisLabel: l, gridLine: g } = chartPalette()
    chart?.setOption({
      legend: { textStyle: { color: l } },
      xAxis: { axisLabel: { color: l }, splitLine: { lineStyle: { color: g } } },
      yAxis: { axisLabel: { color: l }, splitLine: { lineStyle: { color: g } } },
    })
  })
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] })
  window.addEventListener('beforeunload', () => observer.disconnect())
}

function updateChart() {
  if (!chart || history.value.length < 2) return
  chart.setOption({
    xAxis: { data: history.value.map((s) => new Date(s.timestamp * 1000)) },
    series: [
      { data: history.value.map((s) => s.cpu_usage) },
      { data: history.value.map((s) => s.memory_usage_percent) },
      { data: history.value.map((s) => s.disk_usage_percent) },
    ],
  })
}

onMounted(() => {
  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
  ws = new WebSocket(`${protocol}//${location.host}/ws/metrics`)
  ws.onopen = () => {
    wsConnected.value = true
  }
  ws.onclose = () => {
    wsConnected.value = false
  }
  ws.onmessage = (ev: MessageEvent) => {
    const msg = JSON.parse(ev.data)
    if (msg.type === 'init') {
      history.value = msg.data
    } else if (msg.type === 'tick') {
      history.value.push(msg.data)
      if (history.value.length > 60) history.value.shift()
      Object.assign(snap, msg.data)
      lastUpdate.value = new Date().toLocaleString()
    }
    nextTick(updateChart)
  }
  nextTick(initChart)
})

onUnmounted(() => {
  ws?.close()
  chart?.dispose()
})
</script>

<style scoped>
.stat-card {
  height: 140px;
}
.stat {
  display: flex;
  gap: 16px;
  align-items: center;
  height: 100%;
}
.stat-icon {
  width: 56px;
  height: 56px;
  border-radius: var(--radius-lg);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 16px;
  flex-shrink: 0;
}
.icon-cpu {
  background: color-mix(in srgb, var(--brand) 12%, transparent);
  color: var(--brand);
}
.icon-mem {
  background: color-mix(in srgb, #3b82f6 12%, transparent);
  color: #3b82f6;
}
.icon-disk {
  background: color-mix(in srgb, var(--warning) 14%, transparent);
  color: var(--warning);
}
.icon-load {
  background: color-mix(in srgb, var(--success) 12%, transparent);
  color: var(--success);
}
.stat-body {
  flex: 1;
  min-width: 0;
}
.stat-label {
  font-size: 13px;
  color: var(--text-secondary);
  margin-bottom: 2px;
}
.stat-value {
  font-size: 24px;
  font-weight: 700;
  margin-bottom: 6px;
  letter-spacing: -0.02em;
}
.stat-detail {
  font-size: 12px;
  color: var(--text-secondary);
  margin-top: 4px;
}
.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.chart-row {
  margin-top: var(--space-4);
}
.info-row {
  display: flex;
  justify-content: space-between;
  padding: 6px 0;
  font-size: 13px;
  color: var(--text-secondary);
}
</style>
