<template>
  <LayoutContent :title="t('dashboard.title')">
    <!-- 指标卡（点击跳转对应模块） -->
    <div class="stats-grid">
      <button
        v-for="s in statCards"
        :key="s.key"
        class="stat-card"
        :class="[`stat-${s.key}`, { 'is-clickable': s.to }]"
        type="button"
        :aria-label="s.to ? t('dashboard.goTo', { module: s.label }) : s.label"
        @click="s.to && router.push(s.to)"
      >
        <span class="stat-icon">
          <i class="oi" :class="s.icon" />
        </span>
        <span class="stat-body">
          <span class="stat-label">{{ s.label }}</span>
          <span class="stat-value">{{ s.value }}</span>
          <FpProgress :value="s.percent" :style="`height: 5px`" />
          <span class="stat-detail">{{ s.detail }}</span>
        </span>
        <i v-if="s.to" class="oi oi-chevron-right stat-go" aria-hidden="true" />
      </button>
    </div>

    <!-- 图表行 -->
    <div class="charts-grid">
      <div class="panel panel-trend">
        <div class="panel-header">
          <span class="panel-title">{{ t('dashboard.trend') }}</span>
          <span class="ws-badge" :class="{ offline: !wsConnected }">
            <span class="dot" :class="wsConnected ? 'green' : 'red'" />
            {{ wsConnected ? t('dashboard.wsConnected') : t('dashboard.wsDisconnected') }}
          </span>
        </div>
        <div ref="chartRef" class="chart-box" />
      </div>

      <div class="side-stack">
        <div class="panel">
          <div class="panel-header">
            <span class="panel-title">{{ t('dashboard.network') }}</span>
          </div>
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
        </div>
        <div class="panel">
          <div class="info-row">
            <span>{{ t('dashboard.dataPoints') }}</span><span class="mono">{{ history.length }}</span>
          </div>
          <div class="info-row">
            <span>{{ t('dashboard.lastUpdate') }}</span><span class="mono">{{ lastUpdate || '—' }}</span>
          </div>
          <div class="load-chart-title">{{ t('dashboard.load') }}</div>
          <div ref="loadChartRef" class="load-chart" />
        </div>
      </div>
    </div>

    <!-- 下排 -->
    <div class="bottom-grid">
      <div class="panel">
        <div class="panel-header">
          <span class="panel-title">{{ t('dashboard.processTop') }}</span>
        </div>
        <FpTable :rows="topProcesses" :paginator="false" size="small">
          <FpColumn field="name" :header="t('dashboard.processName')" />
          <FpColumn field="cpu" :header="t('dashboard.processCpu')" style="width: 80px">
            <template #body="{ data }">
              <span class="mono">{{ Number(data.cpu).toFixed(1) }}%</span>
            </template>
          </FpColumn>
          <FpColumn field="memory_mb" :header="t('dashboard.processMem')" style="width: 90px">
            <template #body="{ data }">
              <span class="mono">{{ data.memory_mb }} MB</span>
            </template>
          </FpColumn>
          <FpColumn field="pid" :header="t('dashboard.processPid')" style="width: 70px">
            <template #body="{ data }">
              <span class="mono">{{ data.pid }}</span>
            </template>
          </FpColumn>
        </FpTable>
      </div>

      <div class="panel">
        <div class="panel-header">
          <span class="panel-title">{{ t('dashboard.todayTodos') }}</span>
          <FpButtonLink text size="small" @click="router.push('/memos')">{{ t('dashboard.more') }}</FpButtonLink>
        </div>
        <div v-if="todosLoading" class="todo-skeleton">
          <FpSkeleton v-for="i in 4" :key="i" height="28px" />
        </div>
        <div v-else class="todo-list">
          <div
            v-for="td in todayTodos"
            :key="td.id"
            class="todo-item"
            :class="{ done: td.done }"
          >
            <FpCheckbox
              v-if="!isViewer"
              :model-value="td.done"
              :disabled="isViewer"
              @change="(v) => toggleTodo(td, Boolean(v))"
            />
            <span class="todo-content">{{ td.content }}</span>
          </div>
          <div v-if="!todayTodos.length" class="panel-empty">{{ t('dashboard.noTodos') }}</div>
        </div>
      </div>

      <div class="panel">
        <div class="panel-header">
          <span class="panel-title">{{ t('dashboard.commonApps') }}</span>
        </div>
        <div v-if="commonApps.length" class="common-apps">
          <button
            v-for="a in commonApps"
            :key="a.id"
            class="common-app"
            :disabled="isViewer"
            @click="openApp(a)"
          >
            <span class="common-app-icon"><i class="oi oi-box" /></span>
            <span class="common-app-name">{{ a.name }}</span>
            <span class="common-app-count mono">{{ a.launch_count ?? 0 }}</span>
          </button>
        </div>
        <div v-else class="panel-empty">{{ t('dashboard.noCommonApps') }}</div>
      </div>
    </div>
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onUnmounted, nextTick, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { use } from 'echarts/core'
import { LineChart, GaugeChart } from 'echarts/charts'
import { GridComponent, TooltipComponent, LegendComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'





import FpTable from '@/components/ui/FpTable.vue'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import FpButtonLink from '@/components/ui/FpButtonLink.vue'
import FpCheckbox from '@/components/ui/FpCheckbox.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpProgress from '@/components/ui/FpProgress.vue'
import FpSkeleton from '@/components/ui/FpSkeleton.vue'
import { useECharts } from '@/composables/useECharts'
import { useWebSocket } from '@/composables/useWebSocket'
import { usePolling } from '@/composables/usePolling'
import { useQueryCacheClient } from '@/composables/useApiQuery'
import { listTopProcesses } from '@/api/metrics'
import { listMemos, updateMemo } from '@/api/memos'
import { listInstalledApps, launchApp } from '@/api/appStore'
import type { MetricsSnapshot, Memo } from '@/types'
import type { ProcessEntry } from '@/api/generated'
import type { InstalledApp } from '@/api/appStore'

use([LineChart, GaugeChart, GridComponent, TooltipComponent, LegendComponent, CanvasRenderer])

const { t } = useI18n()
const router = useRouter()
const auth = useAuthStore()
/** viewer 只读：隐藏 TODO 勾选与常用应用启动入口 */
const isViewer = computed(() => auth.role === 'viewer')

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

const trendChart = useECharts(chartRef)
const netChart = useECharts(netChartRef)
const loadChart = useECharts(loadChartRef)

let themeObserver: MutationObserver | null = null

const statCards = computed(() => [
  {
    key: 'cpu',
    icon: 'oi-microchip',
    label: t('dashboard.cpu'),
    value: `${snap.cpu_usage.toFixed(1)}%`,
    percent: Math.round(snap.cpu_usage),
    detail: `${snap.cpu_cores} cores`,
    color: usageColor(snap.cpu_usage),
    to: '/nodes',
  },
  {
    key: 'mem',
    icon: 'oi-database',
    label: t('dashboard.memory'),
    value: `${snap.memory_usage_percent.toFixed(1)}%`,
    percent: Math.round(snap.memory_usage_percent),
    detail: `${(snap.memory_used_mb / 1024).toFixed(1)} / ${(snap.memory_total_mb / 1024).toFixed(1)} GB`,
    color: usageColor(snap.memory_usage_percent),
    to: '/nodes',
  },
  {
    key: 'disk',
    icon: 'oi-database' ,
    label: t('dashboard.disk'),
    value: `${snap.disk_usage_percent.toFixed(1)}%`,
    percent: Math.round(snap.disk_usage_percent),
    detail: `${snap.disk_used_gb.toFixed(1)} / ${snap.disk_total_gb.toFixed(1)} GB`,
    color: usageColor(snap.disk_usage_percent),
    to: '/files',
  },
  {
    key: 'load',
    icon: 'oi-wave-pulse',
    label: t('dashboard.load'),
    value: snap.load_one.toFixed(2),
    percent: Math.min(Math.round((snap.load_one / 10) * 100), 100),
    detail: `1m: ${snap.load_one.toFixed(2)} | 5m: ${snap.load_five.toFixed(2)} | 15m: ${snap.load_fifteen.toFixed(2)}`,
    color: 'var(--fp-success)',
    to: '/health',
  },
])

function usageColor(p: number) {
  return p > 80 ? 'var(--fp-danger)' : p > 50 ? 'var(--fp-warning)' : 'var(--fp-success)'
}

// ── 进程 TOP ──
const topProcesses = ref<ProcessEntry[]>([])
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

// ── ECharts 主题自适应 ──
function cssVar(name: string) {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || undefined
}

function chartPalette() {
  return {
    axisLabel: cssVar('--fp-text-secondary') || cssVar('--fp-text-muted'),
    gridLine: cssVar('--fp-border') || cssVar('--fp-border-strong'),
    text: cssVar('--fp-text-primary') || cssVar('--fp-text-secondary'),
    brand: cssVar('--fp-brand'),
    success: cssVar('--fp-success'),
    info: cssVar('--fp-info'),
    warning: cssVar('--fp-warning'),
    danger: cssVar('--fp-danger'),
  }
}

function applyChartTheme() {
  const p = chartPalette()
  trendChart.applyOption({
    legend: { textStyle: { color: p.axisLabel } },
    xAxis: { axisLabel: { color: p.axisLabel }, splitLine: { lineStyle: { color: p.gridLine } } },
    yAxis: { axisLabel: { color: p.axisLabel }, splitLine: { lineStyle: { color: p.gridLine } } },
  })
  netChart.applyOption({
    legend: { textStyle: { color: p.axisLabel } },
    xAxis: { axisLabel: { color: p.axisLabel }, splitLine: { lineStyle: { color: p.gridLine } } },
    yAxis: { axisLabel: { color: p.axisLabel }, splitLine: { lineStyle: { color: p.gridLine } } },
  })
  loadChart.applyOption({
    series: [{ axisLabel: { color: p.axisLabel }, detail: { color: p.axisLabel } }],
  })
}

function initChart() {
  trendChart.mount()
  const { axisLabel, gridLine, brand, success, info } = chartPalette()
  trendChart.applyOption({
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
      { name: 'CPU %', type: 'line', smooth: true, showSymbol: false, lineStyle: { width: 2, color: brand }, itemStyle: { color: brand }, data: [] },
      { name: 'Memory %', type: 'line', smooth: true, showSymbol: false, lineStyle: { width: 2, color: info }, itemStyle: { color: info }, data: [] },
      { name: 'Disk %', type: 'line', smooth: true, showSymbol: false, lineStyle: { width: 2, color: success }, itemStyle: { color: success }, data: [] },
    ],
  })
}

function updateChart() {
  if (!trendChart.instance() || history.value.length < 2) return
  trendChart.setOption(
    {
      xAxis: { data: history.value.map((s) => new Date(s.timestamp * 1000)) },
      series: [
        { data: history.value.map((s) => s.cpu_usage) },
        { data: history.value.map((s) => s.memory_usage_percent) },
        { data: history.value.map((s) => s.disk_usage_percent) },
      ],
    },
    true,
  )
}

// ── 网络 IO 双曲线 ──
function initNetChart() {
  netChart.mount()
  const { axisLabel, gridLine, info, success } = chartPalette()
  netChart.applyOption({
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
      { name: 'RX', type: 'line', smooth: true, showSymbol: false, lineStyle: { width: 1.5, color: info }, itemStyle: { color: info }, data: [] },
      { name: 'TX', type: 'line', smooth: true, showSymbol: false, lineStyle: { width: 1.5, color: success }, itemStyle: { color: success }, data: [] },
    ],
  })
}

function updateNetChart() {
  if (!netChart.instance() || history.value.length < 2) return
  netChart.setOption(
    {
      xAxis: { data: history.value.map((s) => new Date(s.timestamp * 1000)) },
      series: [
        { data: history.value.map((s) => +(s.network_rx_mbps ?? 0).toFixed(3)) },
        { data: history.value.map((s) => +(s.network_tx_mbps ?? 0).toFixed(3)) },
      ],
    },
    true,
  )
}

// ── 负载仪表盘 ──
function initLoadChart() {
  loadChart.mount()
  const { axisLabel, success, warning, danger } = chartPalette()
  loadChart.applyOption({
    tooltip: { trigger: 'item' },
    series: [
      {
        type: 'gauge',
        startAngle: 210,
        endAngle: -30,
        min: 0,
        max: 10,
        radius: '95%',
        axisLine: {
          lineStyle: {
            width: 8,
            color: [
              [0.3, success],
              [0.7, warning],
              [1, danger],
            ],
          },
        },
        pointer: { itemStyle: { color: 'auto' }, length: '55%', width: 4 },
        axisTick: { distance: -8, length: 4, lineStyle: { color: cssVar('--fp-text-invert'), width: 1 } },
        splitLine: { distance: -8, length: 8, lineStyle: { color: cssVar('--fp-text-invert'), width: 2 } },
        axisLabel: { color: axisLabel, distance: 14, fontSize: 9 },
        detail: { valueAnimation: true, formatter: '{value}', color: axisLabel, fontSize: 16, offsetCenter: [0, '55%'] },
        data: [{ value: 0 }],
      },
    ],
  })
}

function updateLoadChart() {
  if (!loadChart.instance()) return
  loadChart.setOption({ series: [{ data: [{ value: +snap.load_one.toFixed(2) }] }] }, true)
}

// ── F1.3 图表性能：1s 节流合并更新 + 不可见不更新 ─────────────────
// 后端 WS 推送间隔可能 < 1s，直接每个 tick 都 setOption 会在低端机产生长任务尖刺。
// 这里统一节流：1s 窗口内只执行一次「合并三图」更新；切到后台（keep-alive 失活）时
// useECharts 已暂停 setOption，这里额外用 visibility 控制跳过无谓计算。
let chartTimer: number | null = null
let chartPending = false

function isChartVisible(): boolean {
  return document.visibilityState === 'visible'
}

function flushCharts() {
  chartTimer = null
  chartPending = false
  if (!isChartVisible()) return
  updateChart()
  updateNetChart()
  updateLoadChart()
}

/** 节流入口：WS tick 调用，最多每 1000ms 合并刷新一次三图 */
function scheduleCharts() {
  if (!isChartVisible()) {
    // 后台：标记待刷新，回到前台时补一次
    chartPending = true
    return
  }
  if (chartTimer === null) {
    chartTimer = window.setTimeout(flushCharts, 1000)
  }
}

// 页面恢复可见时若积压了更新，立即补一次
function onChartVisibility() {
  if (document.visibilityState === 'visible' && chartPending) {
    if (chartTimer !== null) {
      window.clearTimeout(chartTimer)
      chartTimer = null
    }
    flushCharts()
  }
}

// F1.1：WS 消息与查询缓存同步（WS 写缓存，避免双源真相）
const metricsQueryClient = useQueryCacheClient()
function syncMetricsToCache() {
  metricsQueryClient.setQueryData(['metrics', 'history'], history.value)
}

const wsMetrics = useWebSocket('/ws/metrics', {
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
    syncMetricsToCache()
    scheduleCharts()
  },
})

const procPolling = usePolling(refreshProcesses, 10000, { immediate: false })

onMounted(() => {
  wsMetrics.connect()
  nextTick(() => {
    initChart()
    initNetChart()
    initLoadChart()
  })
  themeObserver = new MutationObserver(() => applyChartTheme())
  themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['class', 'style'] })
  document.addEventListener('visibilitychange', onChartVisibility)
  refreshProcesses()
  refreshTodos()
  refreshCommonApps()
  procPolling.start()
})

onUnmounted(() => {
  themeObserver?.disconnect()
  document.removeEventListener('visibilitychange', onChartVisibility)
  if (chartTimer !== null) {
    window.clearTimeout(chartTimer)
    chartTimer = null
  }
})
</script>

<style scoped>
/* 指标卡 */
.stats-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: var(--fp-space-4);
}
.stat-card {
  display: flex;
  gap: var(--fp-space-4);
  align-items: center;
  padding: var(--fp-space-5);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
  transition:
    box-shadow var(--fp-transition-fast),
    transform 120ms var(--fp-ease-out),
    border-color var(--fp-transition-fast);
  /* 重置 button 默认样式 */
  text-align: left;
  font: inherit;
  color: inherit;
  cursor: default;
}
.stat-card.is-clickable {
  cursor: pointer;
}
.stat-card.is-clickable:hover {
  box-shadow: var(--fp-shadow-lg);
  transform: translateY(-1px);
  border-color: var(--fp-brand);
}
.stat-card.is-clickable:focus-visible {
  outline: 2px solid var(--fp-brand);
  outline-offset: 2px;
}
.stat-card.is-clickable:active {
  transform: translateY(0) scale(0.99);
}
.stat-go {
  color: var(--fp-text-muted);
  font-size: 14px;
  flex-shrink: 0;
  transition: color var(--fp-transition-fast), transform var(--fp-transition-fast);
}
.stat-card.is-clickable:hover .stat-go {
  color: var(--fp-brand);
  transform: translateX(2px);
}
.stat-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 52px;
  height: 52px;
  border-radius: var(--fp-radius-md);
  font-size: 22px;
  flex-shrink: 0;
  color: var(--fp-brand);
  background: var(--fp-brand-soft);
}
.stat-mem .stat-icon {
  color: var(--fp-info);
  background: var(--fp-info-soft);
}
.stat-disk .stat-icon {
  color: var(--fp-warning);
  background: var(--fp-warning-soft);
}
.stat-load .stat-icon {
  color: var(--fp-success);
  background: var(--fp-success-soft);
}
.stat-body {
  flex: 1;
  min-width: 0;
}
.stat-label {
  font-size: 12.5px;
  color: var(--fp-text-secondary);
  margin-bottom: 2px;
}
.stat-value {
  font-size: 26px;
  font-weight: 700;
  line-height: 1.3;
  letter-spacing: -0.02em;
  font-variant-numeric: tabular-nums;
  color: var(--fp-text-primary);
}
.stat-detail {
  font-size: 11.5px;
  color: var(--fp-text-muted);
  margin-top: 4px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 面板 */
.panel-empty {
  padding: var(--fp-space-6) var(--fp-space-2);
  text-align: center;
  color: var(--fp-text-muted);
  font-size: 13px;
}

.ws-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--fp-text-secondary);
}
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}
.dot.green {
  background: var(--fp-success);
  box-shadow: 0 0 0 3px var(--fp-success-soft);
}
.dot.red {
  background: var(--fp-danger);
  box-shadow: 0 0 0 3px var(--fp-danger-soft);
}

.charts-grid {
  display: grid;
  grid-template-columns: 1fr 360px;
  gap: var(--fp-space-4);
}
.side-stack {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
}
.chart-box {
  height: 300px;
}
.chart-box-sm {
  height: 170px;
}
.net-values {
  display: flex;
  justify-content: space-between;
  margin-top: var(--fp-space-2);
  font-size: 12px;
  color: var(--fp-text-secondary);
}
.net-dot {
  display: inline-block;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  margin-right: 5px;
}
.net-dot.down {
  background: var(--fp-info);
}
.net-dot.up {
  background: var(--fp-success);
}
.info-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 12.5px;
  color: var(--fp-text-secondary);
  padding: 3px 0;
}
.load-chart-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--fp-text-primary);
  margin-top: var(--fp-space-2);
}
.load-chart {
  height: 130px;
}

/* 下排 */
.bottom-grid {
  display: grid;
  grid-template-columns: 2fr 1fr 1fr;
  gap: var(--fp-space-4);
  align-items: start;
}
.todo-skeleton {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-2);
}
.todo-list {
  display: flex;
  flex-direction: column;
}
.todo-item {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  padding: var(--fp-space-2) 0;
  border-bottom: 1px solid var(--fp-border);
}
.todo-item:last-child {
  border-bottom: none;
}
.todo-content {
  font-size: 13.5px;
  color: var(--fp-text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.todo-item.done .todo-content {
  text-decoration: line-through;
  color: var(--fp-text-muted);
}
.common-apps {
  display: flex;
  flex-direction: column;
}
.common-app {
  display: flex;
  align-items: center;
  gap: var(--fp-space-3);
  padding: var(--fp-space-2);
  border: none;
  background: transparent;
  border-radius: var(--fp-radius-sm);
  cursor: pointer;
  text-align: left;
  font-family: var(--fp-font-sans);
  transition: background-color var(--fp-transition-fast);
}
.common-app:hover {
  background: var(--fp-bg-hover);
}
.common-app-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: var(--fp-radius-sm);
  background: var(--fp-brand-soft);
  color: var(--fp-brand);
  font-size: 15px;
}
.common-app-name {
  flex: 1;
  font-size: 13.5px;
  color: var(--fp-text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.common-app-count {
  font-size: 12px;
  color: var(--fp-text-muted);
}

@media (max-width: 1100px) {
  .stats-grid {
    grid-template-columns: repeat(2, 1fr);
  }
  .charts-grid {
    grid-template-columns: 1fr;
  }
  .bottom-grid {
    grid-template-columns: 1fr 1fr;
  }
}
@media (max-width: 768px) {
  .stats-grid {
    grid-template-columns: 1fr;
  }
  .bottom-grid {
    grid-template-columns: 1fr;
  }
  .chart-box {
    height: 240px;
  }
}
</style>
