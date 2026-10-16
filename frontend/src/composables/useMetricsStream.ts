import { onActivated, onBeforeUnmount, onDeactivated, reactive, ref, shallowRef, watch, type Ref } from 'vue'
import { useWebSocket } from '@/composables/useWebSocket'
import type { MetricsSnapshot } from '@/types'

/** /ws/metrics 消息判别联合——init 携带历史快照数组，tick 携带单条快照（B6） */
export type MetricsWsMessage =
  | { type: 'init'; data: MetricsSnapshot[] }
  | { type: 'tick'; data: MetricsSnapshot }

/** 内存中保留的历史点数上限（超出丢最旧；120 点约等于重载后的首屏曲线长度） */
export const HISTORY_LIMIT = 120

/** 指标流端点：缺省 / null 为本机，正整数为远程节点（Agent 心跳并入的每节点 MetricsHub） */
export function metricsEndpoint(nodeId?: number | null): string {
  return nodeId ? `/ws/metrics?node_id=${nodeId}` : '/ws/metrics'
}

/** 「最后更新」时间格式器：Intl 实例构造开销远大于 format，本模块内缓存一份 */
let timeFormatter: Intl.DateTimeFormat | null = null
function formatNow(): string {
  timeFormatter ??= new Intl.DateTimeFormat(undefined, {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })
  return timeFormatter.format(new Date())
}

/**
 * 仪表盘指标流数据层（Doc/23 §3.3 T2-4 视图拆分第五站；开屏优化二次调整）。
 *
 * 数据来源是 **WebSocket 推送**（`/ws/metrics`），不是轮询——REST 只提供进程表。
 * 语义固化为显式事实：
 *
 * 1. **init 覆盖、tick 追加**：`init` 是连接建立时后端下发的历史（用来立刻画出
 *    一段有形状的曲线），`tick` 是增量。把 tick 也当成覆盖会让曲线永远只有一个点；
 *    把 init 当成追加则每次重连都会把历史拼成两段。
 * 2. **init 同时回填快照**（开屏优化）：旧实现只有 tick 才更新 `snap`，四张指标卡在
 *    连接建立后仍显示 0% 约一个采样周期；init 自带最新一帧，直接取末尾元素回填，
 *    首屏数字与曲线同时出现。
 * 3. **历史封顶 120 点**：WS 长连接下内存无限增长，曲线也会越画越密。超出丢最旧，
 *    保留最近的一段。
 * 4. **`history` 用 `shallowRef` + 整体替换**（开屏优化）：旧实现是深响应数组
 *    `push/shift`，120 个快照 × 14 个字段全部被 Proxy 包裹，且每次 push 触发所有
 *    依赖重算。改为浅响应后快照对象保持原生，消费者（迷你趋势线 / 三图）只在
 *    引用变化时重算一次；每 tick 复制一个 ≤120 元素的数组成本可忽略。
 * 5. **快照对象原地 `Object.assign` 而不是整体替换**：`snap` 是被四张指标卡与
 *    网络面板共同持有的 `reactive` 对象，替换成新对象会让所有已建立的引用
 *    指向旧值（表现为数值再也不更新）。
 * 6. **节点切换**（Doc/30 M-P）：`nodeId` 变化时关闭旧端点订阅、清空历史与快照
 *    （避免把两台机器的曲线拼在一起），再连到 `/ws/metrics?node_id=`；生命周期钩子
 *    在 setup 内只挂一次，作用于当前端点实例。
 */
export function useMetricsStream(options: { onTick?: () => void; nodeId?: Ref<number | null> } = {}) {
  const EMPTY: MetricsSnapshot = {
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
  }
  const snap = reactive<MetricsSnapshot>({ ...EMPTY })

  const history = shallowRef<MetricsSnapshot[]>([])
  const wsConnected = ref(false)
  const lastUpdate = ref('')
  /** 是否已收到首帧（init 或 tick）——首屏骨架 / 占位由此判定，而非依赖连接状态 */
  const hasData = ref(false)

  function applyLatest(latest: MetricsSnapshot | undefined) {
    if (!latest) return
    Object.assign(snap, latest)
    lastUpdate.value = formatNow()
    hasData.value = true
  }

  const handlers = {
    onStatus: (connected: boolean) => {
      wsConnected.value = connected
    },
    onMessage: (data: unknown) => {
      const msg = data as MetricsWsMessage
      if (msg.type === 'init') {
        const arr = Array.isArray(msg.data) ? msg.data : []
        history.value = arr.length > HISTORY_LIMIT ? arr.slice(-HISTORY_LIMIT) : arr
        applyLatest(history.value[history.value.length - 1])
      } else if (msg.type === 'tick') {
        const prev = history.value
        const start = prev.length >= HISTORY_LIMIT ? prev.length - HISTORY_LIMIT + 1 : 0
        const next = prev.slice(start)
        next.push(msg.data)
        history.value = next
        applyLatest(msg.data)
      }
      options.onTick?.()
    },
    manageLifecycle: false,
  }

  const open = (nodeId?: number | null) => useWebSocket(metricsEndpoint(nodeId), handlers)
  let ws = open(options.nodeId?.value)
  let started = false

  function connect() {
    started = true
    ws.connect()
  }

  if (options.nodeId) {
    watch(options.nodeId, (id, prev) => {
      if (id === prev) return
      ws.close()
      Object.assign(snap, EMPTY)
      history.value = []
      hasData.value = false
      wsConnected.value = false
      lastUpdate.value = ''
      ws = open(id)
      if (started) ws.connect()
    })
  }

  onActivated(() => ws.resume())
  onDeactivated(() => ws.pause())
  onBeforeUnmount(() => ws.close())

  return {
    snap,
    history,
    wsConnected,
    lastUpdate,
    hasData,
    connect,
  }
}
