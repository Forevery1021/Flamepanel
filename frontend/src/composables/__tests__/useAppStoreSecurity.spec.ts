import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { createTestI18n } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 拆分的数据层单测（`AppStoreView` → `useAppStoreSecurity`）。
 *
 * 该 composable 只保留「服务端状态 + 读写编排」——编辑态（验签策略下拉、白名单
 * 暂存）归 `StoreSecurityPanel` 本地持有，否则组件必须 mutate 传入对象。
 * 因此这里锁定的是：懒加载语义（首屏不多打接口）、成功回写、失败不改写状态。
 */

const { api, toast } = vi.hoisted(() => ({
  api: { getStoreSecurity: vi.fn(), updateStoreSecurity: vi.fn() },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
}))

vi.mock('@/api/appStore', () => api)
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))

import { useAppStoreSecurity } from '@/composables/useAppStoreSecurity'

const messages = { appStore: { securitySaved: '安全配置已保存' }, common: { failed: '操作失败' } }

const config = {
  signature_policy: 'optional',
  registry_whitelist: ['my.registry'],
  builtin_registries: ['docker.io'],
  effective_registries: ['docker.io', 'my.registry'],
  unsigned_source_count: 1,
}

function makeHost() {
  return defineComponent({
    setup() {
      return { sec: useAppStoreSecurity() }
    },
    template: '<div/>',
  })
}

function mountSecurity() {
  const w = mount(makeHost(), { global: { plugins: [createTestI18n(messages)] } })
  const sec = (w.vm as unknown as { sec: ReturnType<typeof useAppStoreSecurity> }).sec
  return { w, sec }
}

beforeEach(() => {
  vi.clearAllMocks()
  api.getStoreSecurity.mockResolvedValue({ data: config })
})

describe('useAppStoreSecurity', () => {
  it('load 拉取配置并复位 loading', async () => {
    const { sec } = mountSecurity()
    await sec.load()
    expect(sec.config.value?.signature_policy).toBe('optional')
    expect(sec.loading.value).toBe(false)
  })

  it('activate 懒加载：首次激活取一次；config 已就绪时不重复请求', async () => {
    const { sec } = mountSecurity()
    sec.activate()
    await new Promise((r) => setTimeout(r, 10))
    expect(api.getStoreSecurity).toHaveBeenCalledTimes(1)

    sec.activate()
    await new Promise((r) => setTimeout(r, 10))
    expect(api.getStoreSecurity).toHaveBeenCalledTimes(1)
  })

  it('请求进行中重复 activate 不叠加请求（快速来回切页签也不打两枪）', async () => {
    let resolveFn: (v: unknown) => void = () => {}
    api.getStoreSecurity.mockReturnValue(
      new Promise((res) => {
        resolveFn = res
      }),
    )
    const { sec } = mountSecurity()
    sec.activate()
    sec.activate()
    expect(api.getStoreSecurity).toHaveBeenCalledTimes(1)
    resolveFn({ data: config })
    await new Promise((r) => setTimeout(r, 10))
  })

  it('加载失败：不改写 config 且给出错误提示', async () => {
    api.getStoreSecurity.mockRejectedValue(new Error('boom'))
    const { sec } = mountSecurity()
    await sec.load()
    expect(sec.config.value).toBeNull()
    expect(toast.error).toHaveBeenCalled()
    expect(sec.loading.value).toBe(false)
  })

  it('save 成功：载荷原样上报、以响应回写 config、提示成功', async () => {
    api.updateStoreSecurity.mockResolvedValue({
      data: { ...config, signature_policy: 'required', registry_whitelist: [] },
    })
    const { sec } = mountSecurity()
    const ok = await sec.save({ signature_policy: 'required', registry_whitelist: [] })
    expect(ok).toBe(true)
    expect(api.updateStoreSecurity).toHaveBeenCalledWith({
      signature_policy: 'required',
      registry_whitelist: [],
    })
    // 以**服务端响应**为准回写（服务端可能去重/排序，客户端不得自行臆测）
    expect(sec.config.value?.signature_policy).toBe('required')
    expect(sec.config.value?.builtin_registries).toEqual(['docker.io'])
    expect(toast.success).toHaveBeenCalledWith('安全配置已保存')
  })

  it('save 失败：返回 false、保留原 config（保存按钮回到可点态靠 saving 复位）', async () => {
    api.updateStoreSecurity.mockRejectedValue(new Error('500'))
    const { sec } = mountSecurity()
    await sec.load()
    const before = sec.config.value

    const ok = await sec.save({ signature_policy: 'required', registry_whitelist: [] })
    expect(ok).toBe(false)
    expect(sec.config.value).toBe(before)
    expect(sec.saving.value).toBe(false)
  })
})
