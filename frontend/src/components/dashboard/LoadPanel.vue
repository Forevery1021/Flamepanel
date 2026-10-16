<template>
  <div class="panel">
    <div class="info-row">
      <span>{{ t('dashboard.dataPoints') }}</span><span class="mono">{{ dataPoints }}</span>
    </div>
    <div class="info-row">
      <span>{{ t('dashboard.lastUpdate') }}</span><span class="mono">{{ lastUpdate || '—' }}</span>
    </div>
    <div class="load-chart-title">{{ t('dashboard.load') }}</div>
    <div ref="chartEl" class="load-chart" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useECharts } from '@/composables/useECharts'
import { registerDashboardCharts } from './echartsSetup'
import {
  loadInitialOption,
  loadUpdateOption,
  loadThemePatch,
  readChartPalette,
} from '@/utils/dashboardCharts'
import type { MetricsSnapshot } from '@/types'

/**
 * 负载面板（Doc/23 §3.3 T2-4 视图拆分第五站）。
 *
 * 负载仪表盘与两行连接信息（数据点数 / 最后更新）同卡片——它们是同一维度的
 * 事实：**这条实时链路当前有多新鲜**。数据点数持续增长说明 WS 在推，
 * 「最后更新」停住则说明链路静默；把两者放在曲线之外单独显示，运维不必悬停
 * 或看控制台就能判断数据是不是活的（E2E 黄金路径②即靠这两个字段断言 WS 链路）。
 *
 * 与两张折线图的差异：仪表盘只有**单个值**，没有「点数不足不画」的问题，
 * `refreshKey` 一到就更新。空值回落 `—`（不是空串）——空单元格会被读成
 * 「加载失败」。
 */
const props = defineProps<{
  dataPoints: number
  lastUpdate: string
  snapshot: MetricsSnapshot
  refreshKey: number
}>()

registerDashboardCharts()

const chartEl = ref<HTMLElement>()
const chart = useECharts(chartEl)
const { t } = useI18n()

onMounted(() => {
  chart.mount(loadInitialOption(readChartPalette()))
})

watch(
  () => props.refreshKey,
  () => {
    if (!chart.instance()) return
    chart.setOption(loadUpdateOption(props.snapshot), true)
  },
)

function applyTheme() {
  chart.applyOption(loadThemePatch(readChartPalette()))
}

defineExpose({ applyTheme })
</script>

<style scoped>
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
  height: 140px;
}
</style>
