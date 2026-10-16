import { describe, it, expect, vi, beforeEach } from 'vitest'

// 冒烟：项目 Webhook 部署通道 API 层（M-B 主线二 Stage A-7 · Doc/20 §5.8）
// 断言请求路径与方法正确；webhook enable/disable 走常规通道
vi.mock('@/api/client', () => {
  const get = vi.fn()
  const post = vi.fn()
  const put = vi.fn()
  const del = vi.fn()
  return { default: { get, post, put, delete: del } }
})

import api from '@/api/client'
import { enableProjectWebhook, disableProjectWebhook } from '@/api/projects'

describe('projectWebhook API（webhook 部署通道）', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('enableProjectWebhook 走 POST /projects/:id/webhook/enable', async () => {
    ;(api.post as ReturnType<typeof vi.fn>).mockResolvedValue({
      data: { token: 'tok', webhook_url: '/api/projects/3/webhook?token=tok', project: {} },
    })
    await enableProjectWebhook(3)
    expect(api.post).toHaveBeenCalledWith('/projects/3/webhook/enable')
  })

  it('disableProjectWebhook 走 POST /projects/:id/webhook/disable', async () => {
    ;(api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} })
    await disableProjectWebhook(3)
    expect(api.post).toHaveBeenCalledWith('/projects/3/webhook/disable')
  })
})
