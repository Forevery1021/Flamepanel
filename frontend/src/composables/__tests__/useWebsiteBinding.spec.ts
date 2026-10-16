import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { createTestI18n } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第二站的数据层单测（`WebsitesView` → `useWebsiteBinding`）。
 *
 * 这层逻辑原先是 `openBind` / `handleBind` 两个内联函数，混着两个下拉的数据加载
 * 与绑定提交。拆出后本 spec 看住三件事：
 * ① 两个下拉的数据源与过滤（PHP 运行时只留已安装/运行中）；
 * ② **按站点类型择一上报**（另一侧显式 null——这是后端 API 的契约要求，
 *    不是可选优化：不传 null 会把上一轮绑定残留在库里）；
 * ③ 伪静态追加套用的发生时机（仅当选了模板，且必须在绑定成功之后）。
 */

const { api, toast } = vi.hoisted(() => ({
  api: {
    bindWebsiteRuntime: vi.fn(),
    applyWebsiteRewrite: vi.fn(),
    listRuntimes: vi.fn(),
    listProjects: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
}))

vi.mock('@/api/websites', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/websites')>()
  return { ...actual, ...api }
})
vi.mock('@/api/runtimeProject', () => ({
  listRuntimes: api.listRuntimes,
  listProjects: api.listProjects,
}))
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))

import { useWebsiteBinding } from '@/composables/useWebsiteBinding'

const messages = { common: { success: '成功', failed: '操作失败' } }

const runtime = (over: Record<string, unknown> = {}) => ({
  id: 1,
  kind: 'php',
  version: '8.2',
  status: 'installed',
  install_path: '/opt/flame/runtimes/php/8.2',
  is_default: false,
  created_at: '',
  updated_at: '',
  ...over,
})

function mountBinding() {
  const w = mount(
    defineComponent({
      setup() {
        return { binding: useWebsiteBinding() }
      },
      template: '<div/>',
    }),
    { global: { plugins: [createTestI18n(messages)], directives: { permission: () => {} } } },
  )
  return (w.vm as unknown as { binding: ReturnType<typeof useWebsiteBinding> }).binding
}

beforeEach(() => {
  vi.clearAllMocks()
  api.listRuntimes.mockResolvedValue({ data: [runtime()] })
  api.listProjects.mockResolvedValue({ data: [{ id: 5, name: 'my-project' }] })
  api.bindWebsiteRuntime.mockResolvedValue({ data: {} })
  api.applyWebsiteRewrite.mockResolvedValue({ data: '' })
})

describe('useWebsiteBinding · 下拉数据源', () => {
  it('prepare 并行拉取运行时与项目，并暴露为下拉候选', async () => {
    const binding = mountBinding()
    await binding.prepare()

    expect(api.listRuntimes).toHaveBeenCalledTimes(1)
    expect(api.listProjects).toHaveBeenCalledTimes(1)
    expect(binding.phpRuntimeOptions.value).toEqual([{ label: 'php 8.2', value: 1 }])
    expect(binding.projectOptions.value).toEqual([{ label: 'my-project', value: 5 }])
  })

  it('PHP 运行时过滤：非 php 语言与未安装状态一律不进候选', async () => {
    api.listRuntimes.mockResolvedValue({
      data: [
        runtime({ id: 1, kind: 'php', version: '8.2', status: 'installed' }),
        runtime({ id: 2, kind: 'php', version: '8.3', status: 'running' }),
        runtime({ id: 3, kind: 'php', version: '7.4', status: 'not_installed' }),
        runtime({ id: 4, kind: 'node', version: '20', status: 'installed' }),
      ],
    })
    const binding = mountBinding()
    await binding.prepare()

    expect(binding.phpRuntimeOptions.value.map((o) => o.value)).toEqual([1, 2])
  })

  it('每次 prepare 都重取（运维在别处新建的运行时/项目应即时可见）', async () => {
    const binding = mountBinding()
    await binding.prepare()
    api.listRuntimes.mockResolvedValue({ data: [runtime({ id: 9, version: '9.0' })] })
    await binding.prepare()

    expect(api.listRuntimes).toHaveBeenCalledTimes(2)
    expect(binding.phpRuntimeOptions.value.map((o) => o.value)).toEqual([9])
  })

  it('加载失败：toast 报错但不抛异常（弹窗仍可打开，用户可改站点类型后提交）', async () => {
    api.listRuntimes.mockRejectedValue(new Error('boom'))
    const binding = mountBinding()
    await binding.prepare()

    expect(toast.error).toHaveBeenCalled()
    expect(binding.phpRuntimeOptions.value).toEqual([])
  })
})

describe('useWebsiteBinding · 按站点类型择一上报', () => {
  it('runtime_php：上报 runtime_id，project_id 显式 null', async () => {
    const binding = mountBinding()
    await binding.bind(7, {
      site_type: 'runtime_php',
      runtime_id: 3,
      project_id: 99, // 即便误传也应被忽略
    })

    expect(api.bindWebsiteRuntime).toHaveBeenCalledWith(7, {
      site_type: 'runtime_php',
      runtime_id: 3,
      project_id: null,
    })
  })

  it('reverse_proxy：上报 project_id，runtime_id 显式 null', async () => {
    const binding = mountBinding()
    await binding.bind(7, {
      site_type: 'reverse_proxy',
      runtime_id: 3,
      project_id: 99,
    })

    expect(api.bindWebsiteRuntime).toHaveBeenCalledWith(7, {
      site_type: 'reverse_proxy',
      runtime_id: null,
      project_id: 99,
    })
  })

  it('static：两侧都是 null（等价解绑，原实现同语义）', async () => {
    const binding = mountBinding()
    await binding.bind(7, { site_type: 'static', runtime_id: 3, project_id: 99 })

    expect(api.bindWebsiteRuntime).toHaveBeenCalledWith(7, {
      site_type: 'static',
      runtime_id: null,
      project_id: null,
    })
  })

  it('runtime_php 但未选运行时：上报 null（后端按「未绑定」处理）', async () => {
    const binding = mountBinding()
    await binding.bind(7, { site_type: 'runtime_php', runtime_id: null, project_id: null })

    expect(api.bindWebsiteRuntime).toHaveBeenCalledWith(
      7,
      expect.objectContaining({ runtime_id: null }),
    )
  })

  it('成功返回 true（调用方据此关弹窗）', async () => {
    const binding = mountBinding()
    expect(await binding.bind(7, { site_type: 'static', runtime_id: null, project_id: null })).toBe(
      true,
    )
    expect(toast.success).toHaveBeenCalledWith('成功')
  })

  it('失败返回 false 且 toast 报错（弹窗保持打开）', async () => {
    api.bindWebsiteRuntime.mockRejectedValue(new Error('boom'))
    const binding = mountBinding()

    expect(await binding.bind(7, { site_type: 'static', runtime_id: null, project_id: null })).toBe(
      false,
    )
    expect(toast.error).toHaveBeenCalled()
  })
})

describe('useWebsiteBinding · 伪静态追加套用', () => {
  it('选了模板：绑定成功后**追加**一次套用（先绑定后写 vhost）', async () => {
    const order: string[] = []
    api.bindWebsiteRuntime.mockImplementation(async () => {
      order.push('bind')
      return { data: {} }
    })
    api.applyWebsiteRewrite.mockImplementation(async () => {
      order.push('rewrite')
      return { data: '' }
    })

    const binding = mountBinding()
    await binding.bind(7, {
      site_type: 'reverse_proxy',
      runtime_id: null,
      project_id: 5,
      rewrite_preset: 'laravel',
    })

    expect(order).toEqual(['bind', 'rewrite'])
    expect(api.applyWebsiteRewrite).toHaveBeenCalledWith(7, 'laravel')
  })

  it('未选模板：不发套用请求（不能产生无关的 vhost 改写）', async () => {
    const binding = mountBinding()
    await binding.bind(7, { site_type: 'static', runtime_id: null, project_id: null })

    expect(api.applyWebsiteRewrite).not.toHaveBeenCalled()
  })

  it('绑定失败：不执行套用（避免在未绑定的站点上写 vhost）', async () => {
    api.bindWebsiteRuntime.mockRejectedValue(new Error('boom'))
    const binding = mountBinding()
    await binding.bind(7, {
      site_type: 'static',
      runtime_id: null,
      project_id: null,
      rewrite_preset: 'wordpress',
    })

    expect(api.applyWebsiteRewrite).not.toHaveBeenCalled()
  })

  it('套用失败：整体返回 false（调用方不关弹窗，用户可重试）', async () => {
    api.applyWebsiteRewrite.mockRejectedValue(new Error('boom'))
    const binding = mountBinding()

    expect(
      await binding.bind(7, {
        site_type: 'static',
        runtime_id: null,
        project_id: null,
        rewrite_preset: 'wordpress',
      }),
    ).toBe(false)
  })

  it('提交期间 submitting 为 true（防重复提交）', async () => {
    let release: (v: unknown) => void = () => {}
    api.bindWebsiteRuntime.mockReturnValue(new Promise((r) => (release = r)))
    const binding = mountBinding()

    const pending = binding.bind(7, { site_type: 'static', runtime_id: null, project_id: null })
    await new Promise((r) => setTimeout(r, 5))
    expect(binding.submitting.value).toBe(true)

    release({ data: {} })
    await pending
    expect(binding.submitting.value).toBe(false)
  })
})
