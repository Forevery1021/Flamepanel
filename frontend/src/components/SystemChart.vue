<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from 'vue'
import * as echarts from 'echarts'
import type { MetricsSnapshot } from '@/composables/useMetricsWebSocket'

const props = withDefaults(
  defineProps<{
    title: string
    history: MetricsSnapshot[]
    color?: string
    unit?: string
    valueKey?: keyof MetricsSnapshot
    valueKeys?: (keyof MetricsSnapshot)[]
    seriesNames?: string[]
    colors?: string[]
  }>(),
  {
    color: '#409eff',
    unit: '%',
    valueKey: undefined,
    valueKeys: undefined,
    seriesNames: () => [],
    colors: () => [],
  },
)

const chartRef = ref<HTMLDivElement | null>(null)
let chart: echarts.ECharts | null = null
let initialized = false

function buildOption(): echarts.EChartsOption {
  const timestamps = props.history.map((s) => {
    const d = new Date(s.timestamp)
    return d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
  })

  if (props.valueKey) {
    const data = props.history.map((s) => s[props.valueKey!] as number)
    return {
      title: { text: props.title, left: 'center', textStyle: { fontSize: 14 } },
      tooltip: {
        trigger: 'axis',
        valueFormatter: (v: unknown) => (v as number).toFixed(1) + props.unit,
      },
      grid: { left: 50, right: 20, top: 45, bottom: 30 },
      xAxis: { type: 'category', data: timestamps, boundaryGap: false },
      yAxis: { type: 'value', axisLabel: { formatter: `{value}${props.unit}` } },
      series: [
        {
          type: 'line',
          data,
          smooth: true,
          showSymbol: false,
          lineStyle: { color: props.color, width: 2 },
          areaStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: props.color },
              { offset: 1, color: 'rgba(255,255,255,0)' },
            ]),
          },
        },
      ],
    }
  }

  // 多系列模式（负载图）
  const series = (props.valueKeys || []).map((key, i) => ({
    name: props.seriesNames[i] || key,
    type: 'line' as const,
    data: props.history.map((s) => s[key] as number),
    smooth: true,
    showSymbol: false,
    lineStyle: { color: props.colors[i] || '#409eff', width: 2 },
  }))

  return {
    title: { text: props.title, left: 'center', textStyle: { fontSize: 14 } },
    tooltip: { trigger: 'axis' },
    legend: { bottom: 0, data: props.seriesNames },
    grid: { left: 50, right: 20, top: 45, bottom: 30 },
    xAxis: { type: 'category', data: timestamps, boundaryGap: false },
    yAxis: { type: 'value' },
    series,
  }
}

function updateChart() {
  if (!chart || props.history.length === 0) return
  const option = buildOption()
  chart.setOption(option, { notMerge: !initialized })
  initialized = true
}

onMounted(() => {
  if (!chartRef.value) return
  chart = echarts.init(chartRef.value)
  updateChart()
  window.addEventListener('resize', () => chart?.resize())
})

onUnmounted(() => {
  window.removeEventListener('resize', () => chart?.resize())
  chart?.dispose()
  chart = null
  initialized = false
})

watch(() => props.history, updateChart, { deep: true })
</script>

<template>
  <div ref="chartRef" class="system-chart"></div>
</template>

<style scoped>
.system-chart {
  width: 100%;
  height: 250px;
}
</style>
