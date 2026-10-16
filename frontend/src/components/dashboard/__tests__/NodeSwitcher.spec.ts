import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import NodeSwitcher from '@/components/dashboard/NodeSwitcher.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import { createTestI18n, openVuePlugin } from '@/components/ui/__tests__/helpers'

/** Doc/30 M-P：仪表盘节点切换器——本机 + 节点列表、无节点隐藏、无权限不请求。 */

const { listNodes } = vi.hoisted(() => ({ listNodes: vi.fn() }))
vi.mock('@/api/nodes', () => ({ listNodes }))

const messages = { dashboard: { node: { label: '节点', local: '本机' } } }
const settle = () => new Promise((r) => setTimeout(r, 20))

function mountSwitcher(props: { modelValue: number | null; enabled: boolean }) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return mount(NodeSwitcher, {
    props,
    global: {
      plugins: [[VueQueryPlugin, { queryClient: qc }], createTestI18n(messages), openVuePlugin],
    },
  })
}

beforeEach(() => {
  listNodes.mockReset()
})

describe('NodeSwitcher', () => {
  it('列出本机 + 节点；选中节点上抛 ID，选回本机上抛 null', async () => {
    listNodes.mockResolvedValue({
      data: { data: [{ id: 3, name: 'edge', ip_address: '10.0.0.3' }], total: 1 },
    })
    const w = mountSwitcher({ modelValue: null, enabled: true })
    await settle()
    const sel = w.findComponent(FpSelect)
    expect(sel.exists()).toBe(true)
    const options = sel.vm.$attrs.options as { label: string; value: number }[]
    expect(options).toEqual([
      { label: '本机', value: 0 },
      { label: 'edge (10.0.0.3)', value: 3 },
    ])
    expect(sel.props('modelValue')).toBe(0)

    sel.vm.$emit('update:modelValue', 3)
    sel.vm.$emit('update:modelValue', 0)
    expect(w.emitted('update:modelValue')).toEqual([[3], [null]])
  })

  it('没有任何节点时隐藏', async () => {
    listNodes.mockResolvedValue({ data: { data: [], total: 0 } })
    const w = mountSwitcher({ modelValue: null, enabled: true })
    await settle()
    expect(w.findComponent(FpSelect).exists()).toBe(false)
  })

  it('经 URL 直达某节点时即便列表为空也保留切换器（可切回本机）', async () => {
    listNodes.mockResolvedValue({ data: { data: [], total: 0 } })
    const w = mountSwitcher({ modelValue: 8, enabled: true })
    await settle()
    expect(w.findComponent(FpSelect).props('modelValue')).toBe(8)
  })

  it('无 node:read 权限不请求节点列表', async () => {
    mountSwitcher({ modelValue: null, enabled: false })
    await settle()
    expect(listNodes).not.toHaveBeenCalled()
  })
})
