import { useSharedWebSocket, type UseWebSocketOptions, type UseWebSocket } from './useSharedWebSocket'

/**
 * useWebSocket — 统一 WebSocket 生命周期管理（端点级共享连接的兼容入口）。
 *
 * 内部基于 useSharedWebSocket 实现：同一端点的多个组件共享一条连接，
 * 通过引用计数 + 订阅分发避免重复连接（如 Dashboard/Health 同开 /ws/metrics）。
 *
 * 对外 API 保持不变：
 * - 组件卸载自动 close（引用计数减一）
 * - keep-alive 失活（onDeactivated）自动 pause，恢复（onActivated）自动 resume
 * - 连接管理复用 utils/ws.ts（token 附加、指数退避重连）
 */
export function useWebSocket(url: string, options: UseWebSocketOptions): UseWebSocket {
  return useSharedWebSocket(url, options)
}
