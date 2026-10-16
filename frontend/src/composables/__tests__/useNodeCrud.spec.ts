import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第七站的数据层单测（`NodesView` → `useNodeCrud`）。
 *
 * 原实现把注册与编辑写成两套几乎逐行相同的逻辑（两份 reactive、两个校验函数、
 * 两段提交）。合并后本 spec 锁住三件事：
 * ① 两项动作的成功/失败返回语义（true 才关弹窗）；
 * ② 编辑请求体只带四个可编辑字段（Agent 上报的快照不回送）；
 * ③ 删除走二次确认且文案带节点名。
 */

const { api, toast, confirm } = vi.hoisted(() => ({
  api: { createNode: vi.fn(), updateNode: vi.fn(), deleteNode: vi.fn() },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
  confirm: { accept: null as null | (() => Promise<void> | void), action: vi.fn() },
}))

vi.mock('@/api/nodes', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/nodes')>()
  return { ...actual, ...api }
})
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))
vi.mock('@/components/ui/FpConfirm', () => ({
  useFpConfirm: () => ({
    confirmAction: (opts: { accept: () => Promise<void> | void }) => {
      confirm.accept = opts.accept
      confirm.action(opts)
    },
  }),
}))

import { useNodeCrud } from '@/composables/useNodeCrud'
import { initNodeForm } from '@/utils/nodeFormat'

const messages = {
  node: { deleteConfirm: '确定要删除节点 {name} 吗？' },
  common: { success: '操作成功', failed: '操作失败', confirm: '确认' },
}

function mountCrud() {
  const Host = defineComponent({
    setup() {
      return { crud: useNodeCrud() }
    },
    template: '<div/>',
  })
  const w = mount(Host, {
    global: { plugins: [createTestI18n(messages), openVuePlugin, ...services] },
  })
  return (w.vm as unknown as { crud: ReturnType<typeof useNodeCrud> }).crud
}

const form = () => ({ ...initNodeForm(), name: 'n1', hostname: 'h1', ip_address: '10.0.0.1' })

beforeEach(() => {
  vi.clearAllMocks()
  confirm.accept = null
  api.createNode.mockResolvedValue({ data: 1 })
  api.updateNode.mockResolvedValue({ data: {} })
  api.deleteNode.mockResolvedValue({ data: null })
})

describe('useNodeCrud · 注册与编辑', () => {
  it('注册：成功后 toast + 刷新 + 返回 true', async () => {
    const crud = mountCrud()
    const refresh = vi.fn()
    expect(await crud.create(form(), refresh)).toBe(true)
    expect(api.createNode).toHaveBeenCalledWith({
      name: 'n1',
      hostname: 'h1',
      ip_address: '10.0.0.1',
      status: 'online',
    })
    expect(toast.success).toHaveBeenCalled()
    expect(refresh).toHaveBeenCalled()
    expect(crud.submitting.value).toBe(false)
  })

  it('注册失败：toast 错误、返回 false、不刷新（弹窗保持打开）', async () => {
    api.createNode.mockRejectedValue(new Error('名称重复'))
    const crud = mountCrud()
    const refresh = vi.fn()

    expect(await crud.create(form(), refresh)).toBe(false)
    expect(toast.error).toHaveBeenCalled()
    expect(refresh).not.toHaveBeenCalled()
    expect(crud.submitting.value).toBe(false)
  })

  it('【关键】编辑请求体只带四个可编辑字段 + created_at 占位（不回送指标快照）', async () => {
    const crud = mountCrud()
    await crud.save(3, { ...form(), status: 'offline' }, vi.fn())

    const [id, body] = api.updateNode.mock.calls[0] as [number, Record<string, unknown>]
    expect(id).toBe(3)
    expect(body).toEqual({
      id: 3,
      name: 'n1',
      hostname: 'h1',
      ip_address: '10.0.0.1',
      status: 'offline',
      created_at: '',
    })
    expect(body).not.toHaveProperty('metrics_json')
    expect(body).not.toHaveProperty('last_heartbeat_at')
  })

  it('编辑失败：返回 false 且不刷新', async () => {
    api.updateNode.mockRejectedValue(new Error('409'))
    const crud = mountCrud()
    const refresh = vi.fn()
    expect(await crud.save(3, form(), refresh)).toBe(false)
    expect(refresh).not.toHaveBeenCalled()
  })
})

describe('useNodeCrud · 删除', () => {
  it('【关键】删除先二次确认，确认文案带节点名（同名机器不算罕见）', async () => {
    const crud = mountCrud()
    crud.remove({ id: 5, name: 'db-1' } as never, vi.fn())

    const opts = confirm.action.mock.calls[0][0] as { message: string }
    expect(opts.message).toContain('db-1')
    // 未点确认前不发请求
    expect(api.deleteNode).not.toHaveBeenCalled()
  })

  it('确认后发删除请求并刷新', async () => {
    const crud = mountCrud()
    const refresh = vi.fn()
    crud.remove({ id: 5, name: 'db-1' } as never, refresh)

    await confirm.accept?.()
    expect(api.deleteNode).toHaveBeenCalledWith(5)
    expect(refresh).toHaveBeenCalled()
    expect(toast.success).toHaveBeenCalled()
  })

  it('删除失败：toast 错误且不刷新', async () => {
    api.deleteNode.mockRejectedValue(new Error('500'))
    const crud = mountCrud()
    const refresh = vi.fn()
    crud.remove({ id: 5, name: 'db-1' } as never, refresh)

    await confirm.accept?.()
    expect(toast.error).toHaveBeenCalled()
    expect(refresh).not.toHaveBeenCalled()
  })
})
