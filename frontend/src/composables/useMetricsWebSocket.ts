import { ref, onUnmounted } from 'vue'

export interface MetricsSnapshot {
  timestamp: number
  cpu_usage: number
  cpu_cores: number
  memory_usage_percent: number
  memory_total_mb: number
  memory_used_mb: number
  disk_usage_percent: number
  disk_total_gb: number
  disk_used_gb: number
  load_one: number
  load_five: number
  load_fifteen: number
}

export function useMetricsWebSocket() {
  const history = ref<MetricsSnapshot[]>([])
  const connected = ref(false)
  let ws: WebSocket | null = null
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null

  function connect() {
    if (ws && (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING)) {
      return
    }

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    ws = new WebSocket(`${protocol}//${window.location.host}/ws/metrics`)

    ws.onopen = () => {
      connected.value = true
    }

    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data)
        if (msg.type === 'init' && Array.isArray(msg.data)) {
          history.value = msg.data as MetricsSnapshot[]
        } else if (msg.type === 'tick' && msg.data) {
          history.value = [...history.value, msg.data as MetricsSnapshot].slice(-60)
        }
      } catch (e) {
        const preview = typeof event.data === 'string'
          ? event.data.substring(0, 200)
          : String(event.data).substring(0, 200)
        console.error('解析指标数据失败', preview, e)
      }
    }

    ws.onclose = () => {
      connected.value = false
      ws = null
      // 5 秒后自动重连
      reconnectTimer = setTimeout(connect, 5000)
    }

    ws.onerror = () => {
      // onclose 会在 onerror 后触发，由 onclose 负责重连
    }
  }

  function disconnect() {
    if (reconnectTimer) {
      clearTimeout(reconnectTimer)
      reconnectTimer = null
    }
    if (ws) {
      ws.onclose = null // 禁止触发重连
      ws.close()
      ws = null
    }
    connected.value = false
  }

  return { history, connected, connect, disconnect }
}
