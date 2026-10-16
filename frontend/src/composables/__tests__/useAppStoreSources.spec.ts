import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { createTestI18n } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 拆分的数据层单测（`AppStoreView` → `useAppStoreSources`）。
 *
 * 重点看住两处**安全语义**（Doc/20 §4.2）——它们此前埋在 1464 行视图里，
 * 既无单测也无结构性提示，重构时极易被当成「多余的确认弹窗」删掉：
 * ① 未配置公钥的第三方源必须二次确认（拒绝 → 不发请求）；
 * ② 清空公钥 = 卸载信任锚，同样必须二次确认。
 */

const { api, toast, confirmState } = vi.hoisted(() => ({
  api: {
    listSources: vi.fn(),
    createSource: vi.fn(),
    updateSource: vi.fn(),
    deleteSource: vi.fn(),
    syncSource: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
  confirmState: {
    last: null as {
      accept?: () => void
      reject?: () => void
      message: string
    } | null,
  },
}))

vi.mock('@/api/appStore', () => api)
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))
vi.mock('@/components/ui/FpConfirm', () => ({
  useFpConfirm: () => ({
    confirmAction: (opts: { accept?: () => void; reject?: () => void; message: string }) => {
      confirmState.last = opts
    },
  }),
}))

import { useAppStoreSources } from '@/composables/useAppStoreSources'

const messages = {
  appStore: {
    confirmUnsignedSource: '源 {name} 未配置公钥，继续？',
    unsignedSourceTitle: '未签名源',
    sourceCreated: '远程源已添加',
    syncDone: '同步完成：新增 {added}，跳过 {skipped}',
    syncFailed: '同步失败',
    confirmDisable: '停用 {name}？',
    confirmEnable: '启用 {name}？',
    confirmDeleteSource: '删除 {name}？',
    sourceDeleted: '远程源已删除',
    sourceDeleteFailed: '删除失败',
    confirmClearPublicKey: '清空 {name} 的公钥？',
  },
  common: { warning: '警告', success: '成功', failed: '操作失败' },
}

function makeHost() {
  return defineComponent({
    setup() {
      return { sources: useAppStoreSources() }
    },
    template: '<div/>',
  })
}

function mountSources() {
  const w = mount(makeHost(), { global: { plugins: [createTestI18n(messages)] } })
  const src = (w.vm as unknown as { sources: ReturnType<typeof useAppStoreSources> }).sources
  return { w, src }
}

const source = (over: Record<string, unknown> = {}) => ({
  id: 3,
  name: 'official',
  source_type: 'git',
  url: 'https://example.com/appstore',
  public_key: 'KEY',
  enabled: true,
  created_at: '',
  updated_at: '',
  ...over,
})

const payload = { name: 'new', source_type: 'git', url: 'https://x', public_key: '' }

beforeEach(() => {
  vi.clearAllMocks()
  confirmState.last = null
  api.listSources.mockResolvedValue({ data: { total: 0, sources: [] } })
})

describe('useAppStoreSources · 列表', () => {
  it('loadSources 落数据并复位 loading', async () => {
    api.listSources.mockResolvedValue({ data: { total: 1, sources: [source()] } })
    const { src } = mountSources()
    await src.loadSources()
    expect(src.sources.value).toHaveLength(1)
    expect(src.loading.value).toBe(false)
  })

  it('加载失败也复位 loading（不卡死页面）', async () => {
    api.listSources.mockRejectedValue(new Error('boom'))
    const { src } = mountSources()
    await expect(src.loadSources()).rejects.toThrow('boom')
    expect(src.loading.value).toBe(false)
  })
})

describe('useAppStoreSources · 未签名源二次确认（Doc/20 §4.2）', () => {
  it('公钥留空 → 弹确认；**拒绝时不得发出请求**（安全门禁不能被跳过）', async () => {
    const { src } = mountSources()
    const p = src.create(payload)
    expect(confirmState.last?.message).toContain('new')
    expect(api.createSource).not.toHaveBeenCalled()

    confirmState.last?.reject?.()
    await expect(p).resolves.toBe(false)
    expect(api.createSource).not.toHaveBeenCalled()
  })

  it('确认后以 confirm_unsigned=true 提交（后端据此放行）', async () => {
    api.createSource.mockResolvedValue({ data: source() })
    const { src } = mountSources()
    const p = src.create(payload)
    confirmState.last?.accept?.()
    await expect(p).resolves.toBe(true)
    expect(api.createSource).toHaveBeenCalledWith(
      expect.objectContaining({ confirm_unsigned: true, public_key: null }),
    )
    expect(toast.success).toHaveBeenCalledWith('远程源已添加')
  })

  it('已配置公钥 → 直接提交，不弹确认（签名源无需额外风险确认）', async () => {
    api.createSource.mockResolvedValue({ data: source() })
    const { src } = mountSources()
    await expect(
      src.create({ ...payload, public_key: '  ed25519-key  ' }),
    ).resolves.toBe(true)
    expect(confirmState.last).toBeNull()
    expect(api.createSource).toHaveBeenCalledWith(
      expect.objectContaining({ confirm_unsigned: false, public_key: '  ed25519-key  ' }),
    )
  })

  it('名称/地址缺失不发请求（表单前置校验）', async () => {
    const { src } = mountSources()
    await expect(src.create({ ...payload, url: '' })).resolves.toBe(false)
    expect(api.createSource).not.toHaveBeenCalled()
  })

  it('创建失败返回 false（调用方保持弹窗打开）', async () => {
    api.createSource.mockRejectedValue({ response: { data: { detail: 'URL 不可达' } } })
    const { src } = mountSources()
    const p = src.create({ ...payload, public_key: 'k' })
    await expect(p).resolves.toBe(false)
    expect(toast.error).toHaveBeenCalled()
  })
})

describe('useAppStoreSources · 公钥编辑（清空 = 卸载信任锚）', () => {
  it('清空既有公钥 → 弹确认；拒绝时不得提交', async () => {
    const { src } = mountSources()
    src.openKeyDialog(source({ public_key: 'OLD' }) as never)
    const p = src.savePublicKey('')
    expect(confirmState.last?.message).toContain('official')

    confirmState.last?.reject?.()
    await expect(p).resolves.toBe(false)
    expect(api.updateSource).not.toHaveBeenCalled()
  })

  it('确认清空 → public_key=null 且 confirm_unsigned=true', async () => {
    api.updateSource.mockResolvedValue({ data: source() })
    const { src } = mountSources()
    src.openKeyDialog(source({ public_key: 'OLD' }) as never)
    const p = src.savePublicKey('')
    confirmState.last?.accept?.()
    await expect(p).resolves.toBe(true)
    expect(api.updateSource).toHaveBeenCalledWith(3, {
      public_key: null,
      confirm_unsigned: true,
    })
  })

  it('替换为新公钥 → 直接提交，无需确认', async () => {
    api.updateSource.mockResolvedValue({ data: source() })
    const { src } = mountSources()
    src.openKeyDialog(source({ public_key: 'OLD' }) as never)
    await expect(src.savePublicKey('NEW')).resolves.toBe(true)
    expect(confirmState.last).toBeNull()
    expect(api.updateSource).toHaveBeenCalledWith(3, {
      public_key: 'NEW',
      confirm_unsigned: false,
    })
  })

  it('历史未签名源填写公钥 → 同样无需确认（补锚不是降级）', async () => {
    api.updateSource.mockResolvedValue({ data: source() })
    const { src } = mountSources()
    src.openKeyDialog(source({ public_key: null }) as never)
    await expect(src.savePublicKey('NEW')).resolves.toBe(true)
    expect(confirmState.last).toBeNull()
  })

  it('无选中源时不提交（弹窗未打开的防御分支）', async () => {
    const { src } = mountSources()
    await expect(src.savePublicKey('X')).resolves.toBe(false)
    expect(api.updateSource).not.toHaveBeenCalled()
  })
})

describe('useAppStoreSources · 同步 / 启停 / 删除', () => {
  it('同步成功上报计数并刷新；期间 syncingSourceId 置位再复位', async () => {
    api.syncSource.mockResolvedValue({ data: { added: 2, skipped: 1, packages: [] } })
    const { src } = mountSources()
    const p = src.sync(source() as never)
    expect(src.syncingSourceId.value).toBe(3)
    await expect(p).resolves.toBe(true)
    expect(src.syncingSourceId.value).toBeNull()
    expect(toast.success).toHaveBeenCalledWith('同步完成：新增 2，跳过 1')
  })

  it('同步失败返回 false（调用方据此不刷新商店列表）', async () => {
    api.syncSource.mockRejectedValue(new Error('net'))
    const { src } = mountSources()
    await expect(src.sync(source() as never)).resolves.toBe(false)
    expect(src.syncingSourceId.value).toBeNull()
  })

  it('启停：确认后提交取反值', async () => {
    api.updateSource.mockResolvedValue({ data: source() })
    const { src } = mountSources()
    src.toggle(source({ enabled: true }) as never)
    expect(confirmState.last?.message).toContain('停用')
    confirmState.last?.accept?.()
    await new Promise((r) => setTimeout(r, 10))
    expect(api.updateSource).toHaveBeenCalledWith(3, { enabled: false })
  })

  it('删除：确认后调接口并刷新', async () => {
    api.deleteSource.mockResolvedValue({ data: { success: true } })
    const { src } = mountSources()
    src.remove(source() as never)
    expect(confirmState.last?.message).toContain('删除')
    confirmState.last?.accept?.()
    await new Promise((r) => setTimeout(r, 10))
    expect(api.deleteSource).toHaveBeenCalledWith(3)
    expect(toast.success).toHaveBeenCalledWith('远程源已删除')
    expect(api.listSources).toHaveBeenCalled()
  })
})
