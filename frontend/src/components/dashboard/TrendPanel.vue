<template>
  <div class="panel panel-trend">
    <div class="panel-header">
      <span class="panel-title">{{ t('dashboard.trend') }}</span>
      <div class="header-actions">
        <FpSelectButton
          v-if="!rangesDisabled"
          :model-value="range"
          :options="rangeOptions"
          option-label="label"
          option-value="value"
          :allow-empty="false"
          size="small"
          class="range-switch"
          data-tour="dashboard-range"
          :aria-label="t('dashboard.range.label')"
          @update:model-value="onRangeChange"
        />
        <span v-if="range === 'live'" class="ws-badge" :class="{ offline: !wsConnected }">
          <span class="dot" :class="wsConnected ? 'green' : 'red'" />
          {{ wsConnected ? t('dashboard.wsConnected') : t('dashboard.wsDisconnected') }}
        </span>
        <span v-else class="ws-badge" :class="{ offline: historyError }">
          <span class="dot" :class="historyError ? 'red' : historyLoading ? 'amber' : 'blue'" />
          {{
            historyError
              ? t('dashboard.range.loadFailed')
              : t('dashboard.range.avgOf', { step: stepSecs >= 3600 ? '1 h' : '1 min' })
          }}
        </span>
      </div>
    </div>
    <div ref="chartEl" class="chart-box" />
  </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import FpSelectButton from '@/components/ui/FpSelectButton.vue'
import { useECharts } from '@/composables/useECharts'
import { registerDashboardCharts } from './echartsSetup'
import {
  trendInitialOption,
  trendUpdateOption,
  trendThemePatch,
  timeAxisPatch,
  readChartPalette,
  TREND_RANGES,
  type TrendRange,
  type TrendSample,
} from '@/utils/dashboardCharts'

/**
 * 性能趋势面板（Doc/23 §3.3 T2-4 视图拆分第五站）。
 *
 * **图表实例归面板自己持有**：三张图的 option 与生命周期本来互不相干，
 * 原先挤在视图 setup 里只是为了少写几行。拆开后每张图的「数据进入 → 重绘」
 * 链路都在一个文件内可读。
 *
 * **重绘由 `refreshKey` 驱动**（不是直接 watch 数据）：视图层用一份共享的 1s
 * 节流器把 WS tick 合并成一次 `refreshKey++`，三张图各自响应一次。若面板直接
 * watch 数据，每个 tick 都会重绘，节流形同虚设（后端推送间隔可能 < 1s，
 * 低端机上会形成长任务尖刺——F1.3 已记录）。
 *
 * 仅当历史点数 ≥ 2 才重绘：一个点时折线图退化成孤立的点、坐标轴也会退化；
 * 等第二个 tick 再画更稳（原实现即如此，判据随组件一起迁移）。
 *
 * 主题补丁走**增量合并**（`applyOption`）而非整份重建：重建会用首屏
 * 空 option 覆盖已积累的数据点——用户切一次主题就丢历史曲线。
 *
 * S-16 时间范围：面板头部的分段控件在 `live`（WS 实时 6 min）与 `1h / 24h / 7d`
 * （S-07 历史降采样，均值口径）之间切换；范围状态由装配层持有（网络图同步切换），
 * 本面板只负责发出 `update:range` 与按范围打时间轴补丁（刻度格式 / 最小间隔）。
 */
const props = withDefaults(
  defineProps<{
    /** 曲线样本：实时模式为 WS 快照，历史模式为 S-07 降采样点映射（装配层透传） */
    history: readonly TrendSample[]
    /** WS 连接状态（绿点 + 文案，仅实时模式展示） */
    wsConnected: boolean
    /** 节流刷新计数：递增即重绘（由视图层的共享节流器驱动） */
    refreshKey: number
    /** 当前时间范围 */
    range?: TrendRange
    /** 隐藏范围切换（远程节点只有内存实时窗口，无持久化历史） */
    rangesDisabled?: boolean
    /** 历史模式：加载中 / 失败（徽标文案） */
    historyLoading?: boolean
    historyError?: boolean
    /** 历史点桶宽（秒），决定“1 min 均值 / 1 h 均值”文案 */
    stepSecs?: number
  }>(),
  {
    range: 'live',
    rangesDisabled: false,
    historyLoading: false,
    historyError: false,
    stepSecs: 60,
  },
)
const emit = defineEmits<{ 'update:range': [range: TrendRange] }>()

const rangeOptions = computed(() =>
  TREND_RANGES.map((value) => ({ value, label: t(`dashboard.range.${value}`) })),
)

function onRangeChange(value: string | undefined) {
  if (value && (TREND_RANGES as readonly string[]).includes(value)) {
    emit('update:range', value as TrendRange)
  }
}

registerDashboardCharts()

const chartEl = ref<HTMLElement>()
const chart = useECharts(chartEl)
const { t } = useI18n()

onMounted(() => {
  chart.mount(trendInitialOption(readChartPalette()))
  if (props.range !== 'live') chart.applyOption(timeAxisPatch(props.range))
})

watch(
  () => props.refreshKey,
  () => {
    if (!chart.instance() || props.history.length < 2) return
    chart.setOption(trendUpdateOption(props.history), true)
  },
)

// 切换范围：先打时间轴补丁（刻度格式 / 最小间隔），再立即用当前样本重绘
watch(
  () => props.range,
  (range) => {
    if (!chart.instance()) return
    chart.applyOption(timeAxisPatch(range))
    if (props.history.length >= 2) chart.setOption(trendUpdateOption(props.history), true)
  },
)

/** 主题切换时由装配层调用（补丁式，见文件头注释） */
function applyTheme() {
  chart.applyOption(trendThemePatch(readChartPalette()))
}

defineExpose({ applyTheme })
</script>

<style scoped>
.panel-trend {
  min-width: 0;
}
.chart-box {
  height: 100%;
  min-height: 300px;
}
.header-actions {
  display: inline-flex;
  align-items: center;
  gap: var(--fp-space-3);
  min-width: 0;
}
.range-switch {
  flex-shrink: 0;
}
.ws-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--fp-text-secondary);
  white-space: nowrap;
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
.dot.blue {
  background: var(--fp-info);
  box-shadow: 0 0 0 3px var(--fp-info-soft);
}
.dot.amber {
  background: var(--fp-warning);
  box-shadow: 0 0 0 3px var(--fp-warning-soft);
}
</style>
