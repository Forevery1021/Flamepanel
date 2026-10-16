import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { createTestI18n } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第二站的数据层单测（`WebsitesView` → `useWebsiteRewrite`）。
 *
 * 伪静态套用的三处既有语义，此前埋在 1181 行视图里：
 * ① **模板清单以接口 `supported` 为权威**（后端是模板库单一事实来源），
 *    接口未返回时回落本地常量 `REWRITE_PRESETS`——本 spec 两者都锁；
 * ② 写操作后**重取状态**（`rules` 预览要用服务端生成的真实文本，不能用本地拼的）；
 * ③ 套用与清除走不同端点（POST / DELETE），且清除后 `preset` 必须回落 null。
 */

const { api, toast } = vi.hoisted(() => ({
  api: {
    applyWebsiteRewrite: vi.fn(),
    getWebsiteRewrite: vi.fn(),
    clearWebsiteRewrite: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
}))

vi.mock('@/api/websites', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/websites')>()
  return { ...actual, ...api }
})
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))

import { useWebsiteRewrite } from '@/composables/useWebsiteRewrite'
import { REWRITE_PRESETS } from '@/api/websites'

const messages = { common: { success: '成功', failed: '操作失败' } }

const status = (over: Record<string, unknown> = {}) => ({
  preset: 'wordpress',
  rules: 'rewrite ^/index.php$ / last;',
  supported: ['wordpress', 'laravel'],
  ...over,
})

function mountRewrite() {
  const w = mount(
    defineComponent({
      setup() {
        return { rewrite: useWebsiteRewrite() }
      },
      template: '<div/>',
    }),
    { global: { plugins: [createTestI18n(messages)] } },
  )
  return (w.vm as unknown as { rewrite: ReturnType<typeof useWebsiteRewrite> }).rewrite
}

beforeEach(() => {
  vi.clearAllMocks()
  api.getWebsiteRewrite.mockResolvedValue({ data: status() })
  api.applyWebsiteRewrite.mockResolvedValue({ data: '' })
  api.clearWebsiteRewrite.mockResolvedValue({ data: status({ preset: null, rules: '' }) })
})

describe('useWebsiteRewrite · prepare（打开弹窗前拉状态）', () => {
  it('拉取成功：状态落库且原样返回（调用方据此定表单初值）', async () => {
    const rewrite = mountRewrite()
    const res = await rewrite.prepare(7)

    expect(api.getWebsiteRewrite).toHaveBeenCalledWith(7)
    expect(res?.preset).toBe('wordpress')
    expect(rewrite.status.value.rules).toContain('rewrite ^/index.php$')
  })

  it('拉取失败：toast 报错并返回 null（弹窗仍可打开，用户可自行选模板套用）', async () => {
    api.getWebsiteRewrite.mockRejectedValue(new Error('boom'))
    const rewrite = mountRewrite()

    expect(await rewrite.prepare(7)).toBeNull()
    expect(toast.error).toHaveBeenCalled()
    expect(rewrite.status.value.preset).toBeNull()
  })

  it('服务端字段缺失时归一为空值（不把 undefined 透传给模板）', async () => {
    api.getWebsiteRewrite.mockResolvedValue({ data: { supported: ['a'] } })
    const rewrite = mountRewrite()
    await rewrite.prepare(7)

    expect(rewrite.status.value).toEqual({ preset: null, rules: '', supported: ['a'] })
  })
})

describe('useWebsiteRewrite · 模板候选（接口 supported 为权威）', () => {
  it('接口未加载：回落本地常量 REWRITE_PRESETS（下拉不空）', () => {
    const rewrite = mountRewrite()
    expect(rewrite.presetOptions.value.map((o) => o.value)).toEqual([...REWRITE_PRESETS])
  })

  it('接口返回 supported：覆盖本地常量（后端为单一事实来源）', () => {
    const rewrite = mountRewrite()
    rewrite.status.value = status({ supported: ['custom-a', 'custom-b'] })

    expect(rewrite.presetOptions.value).toEqual([
      { label: 'custom-a', value: 'custom-a' },
      { label: 'custom-b', value: 'custom-b' },
    ])
  })
})

describe('useWebsiteRewrite · 套用 / 清除', () => {
  it('apply：写接口后**重取状态**（rules 预览用服务端生成文本）', async () => {
    const order: string[] = []
    api.applyWebsiteRewrite.mockImplementation(async () => {
      order.push('apply')
      return { data: 'ok' }
    })
    api.getWebsiteRewrite.mockImplementation(async () => {
      order.push('get')
      return { data: status({ preset: 'laravel', rules: 'server-generated' }) }
    })

    const rewrite = mountRewrite()
    expect(await rewrite.apply(7, 'laravel')).toBe(true)

    expect(order).toEqual(['apply', 'get'])
    expect(api.applyWebsiteRewrite).toHaveBeenCalledWith(7, 'laravel')
    expect(rewrite.status.value.rules).toBe('server-generated')
    expect(rewrite.status.value.preset).toBe('laravel')
  })

  it('apply：空模板不发请求（下拉未选时点确认不应产生无效调用）', async () => {
    const rewrite = mountRewrite()
    expect(await rewrite.apply(7, '')).toBe(false)
    expect(api.applyWebsiteRewrite).not.toHaveBeenCalled()
  })

  it('apply 失败：返回 false 且错误详情上抛（后端 4xx 的 detail 有诊断价值）', async () => {
    api.applyWebsiteRewrite.mockRejectedValue({
      response: { data: { detail: '站点不支持伪静态' } },
    })
    const rewrite = mountRewrite()

    expect(await rewrite.apply(7, 'wordpress')).toBe(false)
    expect(toast.error).toHaveBeenCalled()
  })

  it('clear：走 DELETE 端点并回写空状态（preset 回落 null）', async () => {
    const rewrite = mountRewrite()
    await rewrite.prepare(7)
    expect(rewrite.status.value.preset).toBe('wordpress')

    expect(await rewrite.clear(7)).toBe(true)
    expect(api.clearWebsiteRewrite).toHaveBeenCalledWith(7)
    expect(rewrite.status.value.preset).toBeNull()
    expect(rewrite.status.value.rules).toBe('')
  })

  it('clear 失败：返回 false（调用方不关弹窗）', async () => {
    api.clearWebsiteRewrite.mockRejectedValue(new Error('boom'))
    const rewrite = mountRewrite()

    expect(await rewrite.clear(7)).toBe(false)
    expect(toast.error).toHaveBeenCalled()
  })

  it('submitting 在写操作期间为 true，结束后复位', async () => {
    let release: (v: unknown) => void = () => {}
    api.applyWebsiteRewrite.mockReturnValue(new Promise((r) => (release = r)))
    const rewrite = mountRewrite()

    const pending = rewrite.apply(7, 'wordpress')
    await new Promise((r) => setTimeout(r, 5))
    expect(rewrite.submitting.value).toBe(true)

    release({ data: '' })
    await pending
    expect(rewrite.submitting.value).toBe(false)
  })
})
