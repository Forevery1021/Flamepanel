<template>
  <LayoutContent :title="t('dashboard.title')" :description="t('pageDesc.dashboard')">
    <template #actions>
      <NodeSwitcher v-model="nodeId" :enabled="canReadNodes" />
    </template>

    <InspectionBanner v-if="canInspect" />

    <StatCardGrid
      :cards="statCards"
      :history="history"
      :pending="!hasData"
      data-tour="dashboard-stats"
      @navigate="router.push"
    />

    <div class="charts-grid">
      <TrendPanel
        ref="trendRef"
        :history="chartSamples"
        :ws-connected="wsConnected"
        :refresh-key="chartRefreshKey"
        :range="effectiveRange"
        :ranges-disabled="nodeId !== null"
        :history-loading="historyLoading"
        :history-error="!!historyError"
        :step-secs="historyStepSecs"
        @update:range="setTrendRange"
      />

      <div class="side-stack">
        <NetworkPanel
          ref="netRef"
          :history="chartSamples"
          :rx-mbps="snap.network_rx_mbps"
          :tx-mbps="snap.network_tx_mbps"
          :refresh-key="chartRefreshKey"
          :range="effectiveRange"
        />
        <LoadPanel
          ref="loadRef"
          :data-points="history.length"
          :last-update="lastUpdate"
          :snapshot="snap"
          :refresh-key="chartRefreshKey"
        />
      </div>
    </div>

    <div class="bottom-grid" :class="{ 'bottom-grid--remote': nodeId !== null }">
      <!-- 进程 TOP 来自本机进程表；远程节点暂无该数据源，选中远程节点时隐藏 -->
      <ProcessPanel v-if="nodeId === null" :rows="processes" />

      <TodoPanel
        :todos="todos"
        :loading="todosLoading"
        :editable="!isViewer"
        @toggle="(todo, done) => toggleTodo(todo, done)"
        @more="router.push('/memos')"
      />

      <CommonAppsPanel :apps="apps" :disabled="isViewer" @open="openApp" />
    </div>
  </LayoutContent>
</template>

<script setup lang="ts">
import { computed, defineAsyncComponent, h, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'

import LayoutContent from '@/components/ui/LayoutContent.vue'
import StatCardGrid from '@/components/dashboard/StatCardGrid.vue'
import ChartSkeleton from '@/components/dashboard/ChartSkeleton.vue'
import ProcessPanel from '@/components/dashboard/ProcessPanel.vue'
import TodoPanel from '@/components/dashboard/TodoPanel.vue'
import CommonAppsPanel from '@/components/dashboard/CommonAppsPanel.vue'
import InspectionBanner from '@/components/dashboard/InspectionBanner.vue'
import NodeSwitcher from '@/components/dashboard/NodeSwitcher.vue'
import { hasPermission } from '@/directives/permission'

import { useMetricsStream } from '@/composables/useMetricsStream'
import { useMetricsHistory } from '@/composables/useMetricsHistory'
import { TREND_RANGES, type TrendRange } from '@/utils/dashboardCharts'
import { useDashboardProcesses } from '@/composables/useDashboardProcesses'
import { useDashboardTodos } from '@/composables/useDashboardTodos'
import { useDashboardApps } from '@/composables/useDashboardApps'
import { useChartThrottle } from '@/composables/useChartThrottle'
import { buildStatCards } from '@/utils/dashboardFormat'

/**
 * 仪表盘视图（Doc/23 §3.3 T2-4 巨型视图拆分第五站）。
 *
 * 拆分前 **851 行**，是 T2-4 队列里形态最特殊的一个：既不是「一页多页签」
 * （AppStore/Docker/Backup），也不是「两个业务域同页」（Websites），而是
 * **一个页面上并列七块面板，其中三块共享同一条实时数据流**——
 * 四张指标卡与三张图全部由 `/ws/metrics` 推送驱动，而进程表（10s 轮询）、
 * 今日待办、常用应用各自独立。
 *
 * 四层结构：
 *
 * | 层 | 文件 | 职责 |
 * |----|------|------|
 * | 数据 | `composables/useMetricsStream.ts` | WS 指标流：init 覆盖并回填快照 / tick 追加 / 120 点封顶 / 浅响应历史 |
 * | 数据 | `composables/useDashboardProcesses.ts` | 进程 TOP（10s 轮询，`immediate: false`） |
 * | 数据 | `composables/useDashboardTodos.ts` | 今日待办（前 6 条、勾选后重拉、失败静默） |
 * | 数据 | `composables/useDashboardApps.ts` | 常用应用（`launch_count` 降序前 6、启动尽力而为） |
 * | 编排 | `composables/useChartThrottle.ts` | 三图共享的 1s 节流 + 可见性补刷 |
 * | 展示 | `components/dashboard/*` | 指标卡网格 + 三图表面板（异步块，含 echarts）+ 三清单面板 + 图表骨架 |
 * | 纯函数 | `utils/dashboardFormat.ts` | 指标卡拼装、使用率配色、MB/s 格式化 |
 * | 纯函数 | `utils/dashboardCharts.ts` | 三图 option 与主题补丁、调色板读取 |
 * | 装配 | 本文件 | 面板接线 + 图表刷新编排（**约 145 行**） |
 *
 * **本站最关键的一处设计**：三张图不各自 watch 数据，而是由本层用一份共享节流器
 * 把 WS tick 合并成一次 `chartRefreshKey++`。若每张图各自响应数据变化，每个 tick
 * 都会重绘三张图，1s 节流形同虚设（后端推送间隔可能 < 1s，低端机上形成长任务尖刺）。
 * 主题切换同理：一次分发到三个面板，而不是各图自己监听 DOM。
 *
 * 行为契约不变：同一批接口、同一文案 key、同一排序/截断口径、同一跳转目标。
 */
/* ── 开屏优化：三块 echarts 面板异步加载，指标卡与清单面板不等 echarts ──
 * echarts 分包（~110KB gzip）只被这三个面板引用；`defineAsyncComponent` 让仪表盘的
 * 同步块只剩指标卡 + 三清单，首帧可在 WS init 到达时立刻有数字与迷你趋势线。
 * `delay: 0` + 等高骨架：图表块就绪前占住版位，避免布局跳动。 */
const chartsLoading = (height: number) => ({
  setup: () => () => h(ChartSkeleton, { height }),
})
const TrendPanel = defineAsyncComponent({
  loader: () => import('@/components/dashboard/TrendPanel.vue'),
  loadingComponent: chartsLoading(360),
  delay: 0,
})
const NetworkPanel = defineAsyncComponent({
  loader: () => import('@/components/dashboard/NetworkPanel.vue'),
  loadingComponent: chartsLoading(240),
  delay: 0,
})
const LoadPanel = defineAsyncComponent({
  loader: () => import('@/components/dashboard/LoadPanel.vue'),
  loadingComponent: chartsLoading(240),
  delay: 0,
})
/** 三个面板都暴露 `applyTheme()`（主题切换时补丁式重刷） */
interface ThemedPanel {
  applyTheme: () => void
}

const { t } = useI18n()
const router = useRouter()
const route = useRoute()
const auth = useAuthStore()
/** viewer 只读：隐藏 TODO 勾选与常用应用启动入口 */
const isViewer = computed(() => auth.role === 'viewer')
/** S-17：安全体检横幅只对持有 `toolbox:read` 的角色订阅（与后端 GET /api/inspection 权限对齐） */
const canInspect = computed(() => hasPermission(auth.role, 'toolbox:read'))
/** Doc/30 M-P：节点切换器（列表需 node:read，与 WS 端点 /ws/metrics 的通道权限一致） */
const canReadNodes = computed(() => hasPermission(auth.role, 'node:read'))

/* ── 节点：`?node=<id>` 选择远程节点（节点列表「监控」入口直达），缺省为本机 ── */

function parseNodeQuery(raw: unknown): number | null {
  const n = typeof raw === 'string' ? Number(raw) : NaN
  return Number.isInteger(n) && n > 0 ? n : null
}
const nodeId = computed<number | null>({
  get: () => parseNodeQuery(route.query.node),
  set: (id) => {
    const query = { ...route.query }
    if (id === null) delete query.node
    else query.node = String(id)
    router.replace({ query })
  },
})

/* ── 三图刷新编排：共享节流器把 WS tick 合并为一次 refreshKey 递增 ── */

const chartRefreshKey = ref(0)
const trendRef = ref<ThemedPanel | null>(null)
const netRef = ref<ThemedPanel | null>(null)
const loadRef = ref<ThemedPanel | null>(null)

const throttle = useChartThrottle(() => {
  chartRefreshKey.value++
})

function applyChartTheme() {
  trendRef.value?.applyTheme()
  netRef.value?.applyTheme()
  loadRef.value?.applyTheme()
}

/* ── 实时指标流（WS）→ 指标卡 + 三图刷新 ── */

const { snap, history, wsConnected, lastUpdate, hasData, connect } = useMetricsStream({
  onTick: () => throttle.schedule(),
  nodeId,
})

const statCards = computed(() => buildStatCards(snap, t))

/* ── S-16 时间范围：live 走 WS 6 min；1h / 24h / 7d 走 S-07 历史（趋势图与网络图同源切换） ── */

const RANGE_STORAGE_KEY = 'fp.dashboard.trendRange'
function readStoredRange(): TrendRange {
  try {
    const v = localStorage.getItem(RANGE_STORAGE_KEY)
    return v && (TREND_RANGES as readonly string[]).includes(v) ? (v as TrendRange) : 'live'
  } catch {
    return 'live'
  }
}
const trendRange = ref<TrendRange>(readStoredRange())
/** 持久化的 1h/24h/7d 历史是本机采样（S-07）；远程节点只有内存中的实时窗口，固定为 live */
const effectiveRange = computed<TrendRange>(() =>
  nodeId.value === null ? trendRange.value : 'live',
)
function setTrendRange(range: TrendRange) {
  trendRange.value = range
  try {
    localStorage.setItem(RANGE_STORAGE_KEY, range)
  } catch {
    /* 隐私模式等存储不可用：仅本次会话生效 */
  }
}

const {
  samples: historySamples,
  loading: historyLoading,
  error: historyError,
  stepSecs: historyStepSecs,
} = useMetricsHistory(effectiveRange)

/** 两张曲线图共用的样本：实时模式为 WS 历史，否则为降采样点 */
const chartSamples = computed(() =>
  effectiveRange.value === 'live' ? history.value : historySamples.value,
)

// 历史数据到达 / 范围切回实时 / 切换节点：与 WS tick 同一条重绘通道（不绕过节流器）
watch([historySamples, effectiveRange, nodeId], () => throttle.schedule())

/* ── 进程 / 待办 / 常用应用 ── */

const { processes, fetch: fetchProcesses, startPolling, stopPolling } = useDashboardProcesses()
const { todos, loading: todosLoading, fetch: fetchTodos, toggle: toggleTodo } = useDashboardTodos()
const { apps, fetch: fetchApps, open: openApp } = useDashboardApps()

/* ── 生命周期 ── */

let themeObserver: MutationObserver | null = null

function onVisibilityChange() {
  // 回到前台若积压了图表更新，立即补刷一次
  throttle.onVisibilityChange()
}

onMounted(() => {
  connect()
  // 首屏各面板数据（进程轮询为 immediate: false，首次数据由此提供）
  fetchProcesses()
  fetchTodos()
  fetchApps()
  startPolling()

  // 主题切换（根元素 class/style 变化）→ 三图补丁式重刷
  themeObserver = new MutationObserver(() => applyChartTheme())
  themeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['class', 'style'],
  })
  document.addEventListener('visibilitychange', onVisibilityChange)
})

onUnmounted(() => {
  themeObserver?.disconnect()
  document.removeEventListener('visibilitychange', onVisibilityChange)
  stopPolling()
  throttle.cancel()
})
</script>

<style scoped>
.charts-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 360px;
  gap: var(--fp-space-4);
  align-items: stretch;
}
.charts-grid > :deep(.panel-trend) {
  display: flex;
  flex-direction: column;
}
.side-stack {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
}
.bottom-grid {
  display: grid;
  grid-template-columns: 2fr 1fr 1fr;
  gap: var(--fp-space-4);
  align-items: start;
}
.bottom-grid--remote {
  grid-template-columns: 1fr 1fr;
}
@media (max-width: 1100px) {
  .charts-grid {
    grid-template-columns: 1fr;
  }
  .bottom-grid {
    grid-template-columns: 1fr 1fr;
  }
}
@media (max-width: 768px) {
  .bottom-grid {
    grid-template-columns: 1fr;
  }
}
</style>
