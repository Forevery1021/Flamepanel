import { describe, it, expect, vi } from 'vitest'
import api from '@/api/client'

vi.mock('@/api/auth', () => ({
  refreshToken: vi.fn(),
  login: vi.fn(),
  logout: vi.fn(),
  fetchMe: vi.fn(),
  changePassword: vi.fn(),
}))

const calls: Array<{ method?: string; url?: string; params?: unknown }> = []

api.defaults.adapter = async (config) => {
  calls.push({ method: config.method, url: config.url, params: config.params })
  return {
    data: { range: '6h', step_secs: 60, from: 0, to: 21600, points: [] },
    status: 200,
    statusText: 'OK',
    headers: {},
    config,
  } as never
}

import { getMetricsHistory, METRICS_HISTORY_RANGES } from '@/api/metrics'
import { queryKeys } from '@/api/queryKeys'

// S-07：监控历史 API 请求形态
describe('metrics history API（S-07）', () => {
  it('getMetricsHistory 走 GET /metrics/history 且以 query 传 range', async () => {
    const res = await getMetricsHistory('6h')
    const c = calls.find((x) => (x.url ?? '').includes('/metrics/history'))
    expect(c?.method).toBe('get')
    expect(c?.params).toEqual({ range: '6h' })
    expect(res.data.step_secs).toBe(60)
  })

  it('缺省 range 为 24h；range 常量与 queryKey 稳定', async () => {
    calls.length = 0
    await getMetricsHistory()
    expect(calls[0]?.params).toEqual({ range: '24h' })
    expect(METRICS_HISTORY_RANGES).toEqual(['1h', '6h', '24h', '7d', '30d'])
    expect(queryKeys.metrics.history('7d')).toEqual(['metrics', 'history', '7d'])
  })
})
