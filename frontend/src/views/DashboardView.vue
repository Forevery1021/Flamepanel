<template>
  <div class="view-container">
    <el-row :gutter="16">
      <el-col :span="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat">
            <div class="stat-icon" style="background:#e6f7ff;color:#1890ff">CPU</div>
            <div class="stat-body">
              <div class="stat-label">{{ t('dashboard.cpu') }}</div>
              <div class="stat-value">{{ snap.cpu_usage.toFixed(1) }}%</div>
              <el-progress :percentage="Math.round(snap.cpu_usage)" :color="cpuColor" :stroke-width="6" />
              <div class="stat-detail">{{ snap.cpu_cores }} cores</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat">
            <div class="stat-icon" style="background:#f0f5ff;color:#2f54eb">MEM</div>
            <div class="stat-body">
              <div class="stat-label">{{ t('dashboard.memory') }}</div>
              <div class="stat-value">{{ snap.memory_usage_percent.toFixed(1) }}%</div>
              <el-progress :percentage="Math.round(snap.memory_usage_percent)" :color="memColor" :stroke-width="6" />
              <div class="stat-detail">{{ (snap.memory_used_mb / 1024).toFixed(1) }} / {{ (snap.memory_total_mb / 1024).toFixed(1) }} GB</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat">
            <div class="stat-icon" style="background:#fff7e6;color:#fa8c16">DSK</div>
            <div class="stat-body">
              <div class="stat-label">{{ t('dashboard.disk') }}</div>
              <div class="stat-value">{{ snap.disk_usage_percent.toFixed(1) }}%</div>
              <el-progress :percentage="Math.round(snap.disk_usage_percent)" :color="diskColor" :stroke-width="6" />
              <div class="stat-detail">{{ snap.disk_used_gb.toFixed(1) }} / {{ snap.disk_total_gb.toFixed(1) }} GB</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat">
            <div class="stat-icon" style="background:#f6ffed;color:#52c41a">LD</div>
            <div class="stat-body">
              <div class="stat-label">{{ t('dashboard.load') }}</div>
              <div class="stat-value">{{ snap.load_one.toFixed(2) }}</div>
              <div class="stat-detail">1m: {{ snap.load_one.toFixed(2) }} | 5m: {{ snap.load_five.toFixed(2) }} | 15m: {{ snap.load_fifteen.toFixed(2) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="16" style="margin-top:16px">
      <el-col :span="16">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>{{ t('dashboard.trend') }}</span>
              <span class="header-tip">{{ t('common.loading') }}</span>
            </div>
          </template>
          <div ref="chartRef" style="height:320px" />
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>WebSocket</span>
              <span class="header-tip">{{ wsConnected ? t('dashboard.wsConnected') : t('dashboard.wsDisconnected') }}</span>
            </div>
          </template>
          <div class="ws-status">
            <span class="dot" :class="wsConnected ? 'green' : 'red'" />
            {{ wsConnected ? t('dashboard.wsConnected') : t('dashboard.wsDisconnected') }}
          </div>
          <el-divider />
          <div class="info-row"><span>{{ t('dashboard.dataPoints') }}</span><span>{{ history.length }}</span></div>
          <div class="info-row"><span>{{ t('dashboard.lastUpdate') }}</span><span>{{ lastUpdate }}</span></div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onUnmounted, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import * as echarts from 'echarts'
import type { MetricsSnapshot } from '@/types'

const { t } = useI18n()

const snap = reactive<MetricsSnapshot>({
  timestamp: 0, cpu_usage: 0, cpu_cores: 0, memory_usage_percent: 0,
  memory_total_mb: 0, memory_used_mb: 0, disk_usage_percent: 0,
  disk_total_gb: 0, disk_used_gb: 0, load_one: 0, load_five: 0, load_fifteen: 0,
})

const history = ref<MetricsSnapshot[]>([])
const chartRef = ref<HTMLElement>()
const wsConnected = ref(false)
const lastUpdate = ref('')
let chart: echarts.ECharts | null = null
let ws: WebSocket | null = null

function cpuColor(p: number) { return p > 80 ? '#f56c6c' : p > 50 ? '#e6a23c' : '#67c23a' }
const memColor = cpuColor
const diskColor = cpuColor

function initChart() {
  if (!chartRef.value) return
  chart = echarts.init(chartRef.value)
  chart.setOption({
    tooltip: { trigger: 'axis' },
    legend: { data: ['CPU %', 'Memory %', 'Disk %'], bottom: 0, textStyle: { color: '#909399' } },
    grid: { left: 50, right: 20, top: 20, bottom: 40 },
    xAxis: { type: 'time', axisLabel: { fontSize: 11 } },
    yAxis: { type: 'value', max: 100, axisLabel: { fontSize: 11 } },
    series: [
      { name: 'CPU %', type: 'line', smooth: true, showSymbol: false, lineStyle: { width: 2 }, data: [] },
      { name: 'Memory %', type: 'line', smooth: true, showSymbol: false, lineStyle: { width: 2 }, data: [] },
      { name: 'Disk %', type: 'line', smooth: true, showSymbol: false, lineStyle: { width: 2 }, data: [] },
    ],
  })
}

function updateChart() {
  if (!chart || history.value.length < 2) return
  chart.setOption({
    xAxis: { data: history.value.map(s => new Date(s.timestamp * 1000)) },
    series: [
      { data: history.value.map(s => s.cpu_usage) },
      { data: history.value.map(s => s.memory_usage_percent) },
      { data: history.value.map(s => s.disk_usage_percent) },
    ],
  })
}

onMounted(() => {
  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
  ws = new WebSocket(`${protocol}//${location.host}/ws/metrics`)
  ws.onopen = () => { wsConnected.value = true }
  ws.onclose = () => { wsConnected.value = false }
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
.stat-card { height: 140px; }
.stat { display: flex; gap: 16px; align-items: center; height: 100%; }
.stat-icon { width: 56px; height: 56px; border-radius: 12px; display: flex; align-items: center; justify-content: center; font-weight: 700; font-size: 16px; flex-shrink: 0; }
.stat-body { flex: 1; min-width: 0; }
.stat-label { font-size: 13px; color: #909399; margin-bottom: 2px; }
.stat-value { font-size: 24px; font-weight: 700; margin-bottom: 6px; }
.stat-detail { font-size: 12px; color: #909399; margin-top: 4px; }
.card-header { display: flex; align-items: center; justify-content: space-between; }
.header-tip { font-size: 12px; color: #909399; background: #f5f7fa; padding: 2px 8px; border-radius: 4px; }
.dark .header-tip { background: #2c2d2e; }
.ws-status { display: flex; align-items: center; gap: 8px; font-size: 14px; }
.dot { width: 10px; height: 10px; border-radius: 50%; display: inline-block; }
.dot.green { background: #67c23a; }
.dot.red { background: #f56c6c; }
.info-row { display: flex; justify-content: space-between; padding: 4px 0; font-size: 13px; color: #606266; }
</style>
