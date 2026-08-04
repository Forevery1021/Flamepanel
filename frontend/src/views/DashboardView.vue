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
            <div class="stat-icon icon-mem"><el-icon :size="26"><MemoIcon /></el-icon></div>
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
      <el-col :xs="24" :lg="12">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>{{ t('dashboard.trend') }}</span>
            </div>
          </template>
          <div ref="chartRef" class="chart-box" />
        </el-card>
      </el-col>
      <el-col :xs="24" :lg="6">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>{{ t('dashboard.network') }}</span>
            </div>
          </template>
          <div ref="netChartRef" class="chart-box chart-box-sm" />
          <div class="net-values">
            <span class="net-val">
              <span class="net-dot down" />{{ t('dashboard.rx') }}
              {{ snap.network_rx_mbps.toFixed(2) }} MB/s
            </span>
            <span class="net-val">
              <span class="net-dot up" />{{ t('dashboard.tx') }}
              {{ snap.network_tx_mbps.toFixed(2) }} MB/s
            </span>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :lg="6">
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
          <el-divider />
          <div class="load-chart-title">{{ t('dashboard.load') }}</div>
          <div ref="loadChartRef" class="load-chart" />
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="16" class="chart-row">
      <el-col :xs="24" :lg="10">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>{{ t('dashboard.processTop') }}</span>
            </div>
          </template>
          <el-table :data="topProcesses" size="small" :empty-text="t('common.noData')">
            <el-table-column prop="name" :label="t('dashboard.processName')" min-width="120" show-overflow-tooltip />
            <el-table-column prop="cpu" :label="t('dashboard.processCpu')" width="70">
              <template #default="{ row }">{{ row.cpu.toFixed(1) }}%</template>
            </el-table-column>
            <el-table-column prop="memory_mb" :label="t('dashboard.processMem')" width="80">
              <template #default="{ row }">{{ row.memory_mb }} MB</template>
            </el-table-column>
            <el-table-column prop="pid" :label="t('dashboard.processPid')" width="60" />
          </el-table>
        </el-card>
      </el-col>
      <el-col :xs="24" :lg="8">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>{{ t('dashboard.todayTodos') }}</span>
              <el-button text size="small" @click="$router.push('/memos')">{{ t('dashboard.more') }}</el-button>
            </div>
          </template>
          <div v-loading="todosLoading" class="todo-list">
            <div v-for="td in todayTodos" :key="td.id" class="todo-item" :class="{ done: td.done }">
              <el-checkbox
                :model-value="td.done"
                @change="(val: string | number | boolean) => toggleTodo(td, Boolean(val))"
              />
              <span class="todo-content">{{ td.content }}</span>
            </div>
            <div v-if="!todayTodos.length" class="todo-empty">{{ t('dashboard.noTodos') }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :lg="6">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>{{ t('dashboard.commonApps') }}</span>
            </div>
          </template>
          <div v-if="commonApps.length" class="common-apps">
            <div
              v-for="a in commonApps"
              :key="a.id"
              class="common-app"
              @click="openApp(a)"
            >
              <el-icon><Box /></el-icon>
              <span class="common-app-name">{{ a.name }}</span>
              <span class="common-app-count">{{ a.launch_count ?? 0 }}</span>
            </div>
          </div>
          <div v-else class="todo-empty">{{ t('dashboard.noCommonApps') }}</div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onUnmounted, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { init, use } from 'echarts/core'
import { LineChart, GaugeChart } from 'echarts/charts'
import { GridComponent, TooltipComponent, LegendComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import { Cpu, Memo as MemoIcon, Coin, TrendCharts, Box } from '@element-plus/icons-vue'
import { connectWithRetry } from '@/utils/ws'
import { listTopProcesses } from '@/api/metrics'
import { listMemos, updateMemo } from '@/api/memos'
import { listInstalledApps, launchApp } from '@/api/appStore'
import type { MetricsSnapshot, ProcessEntry, Memo } from '@/types'
import type { InstalledApp } from '@/api/appStore'
import type { ECharts } from 'echarts/core'

use([LineChart, GaugeChart, GridComponent, TooltipComponent, LegendComponent, CanvasRenderer])

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
  network_rx_mbps: 0,
  network_tx_mbps: 0,
})

const history = ref<MetricsSnapshot[]>([])
const chartRef = ref<HTMLElement>()
const netChartRef = ref<HTMLElement>()
const loadChartRef = ref<HTMLElement>()
const wsConnected = ref(false)
const lastUpdate = ref('')
let chart: ECharts | null = null
let netChart: ECharts | null = null
let loadChart: ECharts | null = null
let wsConn: ReturnType<typeof connectWithRetry> | null = null

// ── 进程 TOP ──
const topProcesses = ref<ProcessEntry[]>([])
let procTimer: number | null = null
async function refreshProcesses() {
  try {
    const res = await listTopProcesses()
    topProcesses.value = res.data
  } catch {
    topProcesses.value = []
  }
}

// ── 今日 TODO ──
const todayTodos = ref<Memo[]>([])
const todosLoading = ref(false)
async function refreshTodos() {
  todosLoading.value = true
  try {
    const res = await listMemos('todo', false)
    todayTodos.value = res.data.slice(0, 6)
  } catch {
    todayTodos.value = []
  } finally {
    todosLoading.value = false
  }
}
async function toggleTodo(m: Memo, done: boolean) {
  try {
    await updateMemo(m.id, { done })
    m.done = done
    refreshTodos()
  } catch {
    // ignore
  }
}

// ── 常用应用 ──
const commonApps = ref<InstalledApp[]>([])
async function refreshCommonApps() {
  try {
    const res = await listInstalledApps()
    commonApps.value = [...res.data]
      .sort((a, b) => (b.launch_count ?? 0) - (a.launch_count ?? 0))
      .slice(0, 6)
  } catch {
    commonApps.value = []
  }
}
function openApp(a: InstalledApp) {
  launchApp(a.id).catch(() => {})
  if (a.access_url) window.open(a.access_url, '_blank')
}

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

// ── 网络 IO 双曲线 ──
function initNetChart() {
  if (!netChartRef.value) return
  netChart = init(netChartRef.value)
  const { axisLabel, gridLine } = chartPalette()
  netChart.setOption({
    tooltip: { trigger: 'axis' },
    legend: { data: ['RX', 'TX'], bottom: 0, textStyle: { color: axisLabel } },
    grid: { left: 44, right: 12, top: 16, bottom: 36 },
    xAxis: {
      type: 'time',
      axisLabel: { fontSize: 10, color: axisLabel },
      splitLine: { lineStyle: { color: gridLine } },
    },
    yAxis: {
      type: 'value',
      axisLabel: { fontSize: 10, color: axisLabel },
      splitLine: { lineStyle: { color: gridLine } },
    },
    series: [
      { name: 'RX', type: 'line', smooth: true, showSymbol: false, lineStyle: { width: 1.5, color: '#3b82f6' }, itemStyle: { color: '#3b82f6' }, data: [] },
      { name: 'TX', type: 'line', smooth: true, showSymbol: false, lineStyle: { width: 1.5, color: '#10b981' }, itemStyle: { color: '#10b981' }, data: [] },
    ],
  })
}

function updateNetChart() {
  if (!netChart || history.value.length < 2) return
  netChart.setOption({
    xAxis: { data: history.value.map((s) => new Date(s.timestamp * 1000)) },
    series: [
      { data: history.value.map((s) => +(s.network_rx_mbps ?? 0).toFixed(3)) },
      { data: history.value.map((s) => +(s.network_tx_mbps ?? 0).toFixed(3)) },
    ],
  })
}

// ── 负载迷你图 ──
function initLoadChart() {
  if (!loadChartRef.value) return
  loadChart = init(loadChartRef.value)
  const { axisLabel } = chartPalette()
  loadChart.setOption({
    tooltip: { trigger: 'item' },
    series: [
      {
        type: 'gauge',
        startAngle: 210,
        endAngle: -30,
        min: 0,
        max: 10,
        radius: '95%',
        axisLine: { lineStyle: { width: 8, color: [[0.3, '#67c23a'], [0.7, '#e6a23c'], [1, '#f56c6c']] } },
        pointer: { itemStyle: { color: 'auto' }, length: '55%', width: 4 },
        axisTick: { distance: -8, length: 4, lineStyle: { color: '#fff', width: 1 } },
        splitLine: { distance: -8, length: 8, lineStyle: { color: '#fff', width: 2 } },
        axisLabel: { color: axisLabel, distance: 14, fontSize: 9 },
        detail: { valueAnimation: true, formatter: '{value}', color: axisLabel, fontSize: 16, offsetCenter: [0, '55%'] },
        data: [{ value: 0 }],
      },
    ],
  })
}

function updateLoadChart() {
  if (!loadChart) return
  loadChart.setOption({ series: [{ data: [{ value: +snap.load_one.toFixed(2) }] }] })
}

onMounted(() => {
  wsConn = connectWithRetry('/ws/metrics', {
    onStatus: (connected) => {
      wsConnected.value = connected
    },
    onMessage: (data) => {
      const msg = data as { type: string; data: MetricsSnapshot }
      if (msg.type === 'init') {
        history.value = msg.data as unknown as MetricsSnapshot[]
      } else if (msg.type === 'tick') {
        history.value.push(msg.data)
        if (history.value.length > 120) history.value.shift()
        Object.assign(snap, msg.data)
        lastUpdate.value = new Date().toLocaleString()
      }
      nextTick(() => {
        updateChart()
        updateNetChart()
        updateLoadChart()
      })
    },
  })
  nextTick(() => {
    initChart()
    initNetChart()
    initLoadChart()
  })
  refreshProcesses()
  refreshTodos()
  refreshCommonApps()
  procTimer = window.setInterval(refreshProcesses, 10000)
})

onUnmounted(() => {
  wsConn?.close()
  chart?.dispose()
  netChart?.dispose()
  loadChart?.dispose()
  if (procTimer !== null) clearInterval(procTimer)
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
.chart-box-sm {
  height: 200px;
}
.net-values {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
  color: var(--text-secondary);
  padding: 4px 0;
}
.net-val {
  display: flex;
  align-items: center;
  gap: 4px;
}
.net-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  display: inline-block;
}
.net-dot.down {
  background: #3b82f6;
}
.net-dot.up {
  background: #10b981;
}
.load-chart-title {
  font-size: 13px;
  color: var(--text-secondary);
  margin-bottom: 4px;
}
.load-chart {
  height: 120px;
}
.todo-list {
  min-height: 140px;
}
.todo-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 4px;
  font-size: 13px;
}
.todo-item.done .todo-content {
  text-decoration: line-through;
  color: var(--text-muted);
}
.todo-content {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.todo-empty {
  padding: 30px 0;
  text-align: center;
  color: var(--text-muted);
  font-size: 13px;
}
.common-apps {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.common-app {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: 13px;
  color: var(--text-primary);
}
.common-app:hover {
  background: var(--bg-hover);
}
.common-app-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.common-app-count {
  font-size: 11px;
  color: var(--text-muted);
  background: var(--bg-hover);
  border-radius: 8px;
  padding: 1px 8px;
}
</style>
