<template>
  <div class="panel">
    <div class="panel-header">
      <span class="panel-title">{{ t('dashboard.network') }}</span>
    </div>
    <div ref="chartEl" class="chart-box chart-box-sm" />
    <div class="net-values">
      <span class="net-val">
        <span class="net-dot down" />{{ t('dashboard.rx') }} {{ formatMbps(rxMbps) }} MB/s
      </span>
      <span class="net-val">
        <span class="net-dot up" />{{ t('dashboard.tx') }} {{ formatMbps(txMbps) }} MB/s
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useECharts } from '@/composables/useECharts'
import { registerDashboardCharts } from './echartsSetup'
import {
  netInitialOption,
  netUpdateOption,
  netThemePatch,
  timeAxisPatch,
  readChartPalette,
  type TrendRange,
  type TrendSample,
} from '@/utils/dashboardCharts'
import { formatMbps } from '@/utils/dashboardFormat'

/**
 * 网络 IO 面板（Doc/23 §3.3 T2-4 视图拆分第五站）。
 *
 * 双曲线图 + 图例下方的两个实时读数（下行/上行 MB/s）。读数与曲线同屏是刻意的：
 * 曲线看趋势、读数看当下值——只有曲线时运维要悬停才能读数，只有读数时看不出突刺。
 *
 * 读数走 `formatMbps`（2 位小数），曲线数据点保留 **3 位小数**（`netUpdateOption`
 * 内）——MB/s 常见量级在 0.001–1，两位小数会把轻载机器的曲线压成一条 0 线，
 * 而卡片读数两位已足够。这处「同源数据、不同精度」是原实现的取舍，拆出后
 * 各自都有明确落点（读数在 `dashboardFormat`、曲线在 `dashboardCharts`）。
 *
 * 重绘时机与主题补丁同 `TrendPanel`（受 `refreshKey` 驱动、增量补丁不重建）。
 */
const props = withDefaults(
  defineProps<{
    /** 曲线样本：实时为 WS 快照，历史模式为 S-07 降采样点映射（与趋势图同源同范围） */
    history: readonly TrendSample[]
    rxMbps: number
    txMbps: number
    refreshKey: number
    /** 当前时间范围（S-16，与趋势图同步；只影响时间轴刻度） */
    range?: TrendRange
  }>(),
  { range: 'live' },
)

registerDashboardCharts()

const chartEl = ref<HTMLElement>()
const chart = useECharts(chartEl)
const { t } = useI18n()

onMounted(() => {
  chart.mount(netInitialOption(readChartPalette()))
  if (props.range !== 'live') chart.applyOption(timeAxisPatch(props.range, true))
})

watch(
  () => props.refreshKey,
  () => {
    if (!chart.instance() || props.history.length < 2) return
    chart.setOption(netUpdateOption(props.history), true)
  },
)

watch(
  () => props.range,
  (range) => {
    if (!chart.instance()) return
    chart.applyOption(timeAxisPatch(range, true))
    if (props.history.length >= 2) chart.setOption(netUpdateOption(props.history), true)
  },
)

function applyTheme() {
  chart.applyOption(netThemePatch(readChartPalette()))
}

defineExpose({ applyTheme })
</script>

<style scoped>
.chart-box {
  height: 160px;
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
</style>
