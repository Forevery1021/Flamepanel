import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第六站的数据层单测（`ProjectView` → `useProjectList`）。
 *
 * 这层逻辑原先是「832 行巨型视图内的内联函数」——既在 `src/views/**`（覆盖率
 * 排除区），又没人能脱离整页挂载去验证它。拆出后本 spec 直接看住四件事：
 * ① 列表查询与刷新；② 客户端搜索的两字段口径（名称 / 域名，且**不含根路径**）；
 * ③ 增改的请求体组装（创建带 name、更新不带、空串归一为 null）；
 * ④ 运行时候选按 kind 去重。
 */

const { api, toast } = vi.hoisted(() => ({
  api: {
    listProjects: vi.fn(),
    createProject: vi.fn(),
    updateProject: vi.fn(),
    listRuntimes: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
}))

vi.mock('@/api/projects', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/projects')>()
  return {
    ...actual,
    listProjects: api.listProjects,
    createProject: api.createProject,
    updateProject: api.updateProject,
  }
})
vi.mock('@/api/runtimes', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/runtimes')>()
  return { ...actual, listRuntimes: api.listRuntimes }
})
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))
vi.mock('@/components/ui/FpConfirm', () => ({ useFpConfirm: () => ({ confirmAction: vi.fn() }) }))

import { useProjectList } from '@/composables/useProjectList'
import { initProjectForm } from '@/utils/projectFormat'

const messages = {
  project: { createSuccess: '项目创建成功', updateSuccess: '项目更新成功' },
  common: { failed: '操作失败' },
}

const row = (over: Record<string, unknown> = {}) => ({
  id: 1,
  name: 'blog',
  kind: 'node',
  source: 'git',
  source_url: 'https://example.com/blog.git',
  runtime_id: 7,
  run_command: 'npm start',
  port: 3000,
  domain: 'blog.example.com',
  health_check_path: '/health',
  deploy_strategy: 'rolling',
  status: 'running',
  deploy_type: 'manual',
  build_type: 'dockerfile',
  deploy_path: '/opt/x',
  has_procfile: false,
  has_webhook_token: false,
  auto_deploy: false,
  created_at: '',
  updated_at: '',
  ...over,
})

function mountList() {
  const Host = defineComponent({
    setup() {
      return { list: useProjectList() }
    },
    template: '<div/>',
  })
  const w = mount(Host, {
    global: {
      plugins: [
        createTestI18n(messages),
        openVuePlugin,
        ...services,
        [
          VueQueryPlugin,
          { queryClient: new QueryClient({ defaultOptions: { queries: { retry: false } } }) },
        ],
      ],
      directives: { permission: () => {} },
    },
  })
  return (w.vm as unknown as { list: ReturnType<typeof useProjectList> }).list
}

async function settle() {
  await new Promise((r) => setTimeout(r, 20))
}

const form = (over: Record<string, unknown> = {}) => ({
  ...initProjectForm(),
  name: 'blog',
  ...over,
})

beforeEach(() => {
  vi.clearAllMocks()
  api.listProjects.mockResolvedValue({ data: [row()] })
  api.listRuntimes.mockResolvedValue({
    data: [
      {
        id: 7,
        kind: 'node',
        version: '20',
        status: 'installed',
        install_path: '/opt/node',
        is_default: true,
        created_at: '',
        updated_at: '',
      },
    ],
  })
  api.createProject.mockResolvedValue({ data: row() })
  api.updateProject.mockResolvedValue({ data: row() })
})

describe('useProjectList · 列表与搜索', () => {
  it('挂载即拉列表，并映射为 projects', async () => {
    const ctl = mountList()
    await settle()
    expect(api.listProjects).toHaveBeenCalledTimes(1)
    expect(ctl.projects.value).toHaveLength(1)
  })

  it('空查询返回原数组（不做多余过滤）', async () => {
    api.listProjects.mockResolvedValue({ data: [row(), row({ id: 2, name: 'api', domain: null })] })
    const ctl = mountList()
    await settle()
    expect(ctl.filteredProjects.value).toHaveLength(2)
  })

  it('【关键】搜索匹配名称与域名两字段，大小写不敏感', async () => {
    api.listProjects.mockResolvedValue({
      data: [row({ name: 'blog' }), row({ id: 2, name: 'api', domain: 'API.example.com' })],
    })
    const ctl = mountList()
    await settle()

    ctl.search.value = 'API'
    expect(ctl.filteredProjects.value.map((p) => p.id)).toEqual([2])

    ctl.search.value = 'blo'
    expect(ctl.filteredProjects.value.map((p) => p.id)).toEqual([1])
  })

  it('【口径】搜索**不匹配根路径**（原实现只搜名称与域名）', async () => {
    api.listProjects.mockResolvedValue({
      data: [row({ name: 'blog', domain: 'blog.example.com' })],
    })
    const ctl = mountList()
    await settle()

    ctl.search.value = 'deploy-path'
    expect(ctl.filteredProjects.value).toHaveLength(0)
  })

  it('running 无关搜索：domain 为 null 的行不炸（`?? ""` 归一）', async () => {
    api.listProjects.mockResolvedValue({ data: [row({ domain: null })] })
    const ctl = mountList()
    await settle()
    ctl.search.value = 'blo'
    expect(ctl.filteredProjects.value).toHaveLength(1)
  })
})

describe('useProjectList · 创建与编辑', () => {
  it('创建：请求体带 name，成功后刷新并返回 true', async () => {
    const ctl = mountList()
    await settle()
    const before = api.listProjects.mock.calls.length

    const ok = await ctl.create(form({ name: 'blog', port: 8080 }) as never)
    expect(ok).toBe(true)
    expect(api.createProject).toHaveBeenCalledWith(
      expect.objectContaining({ name: 'blog', port: 8080 }),
    )
    expect(api.listProjects.mock.calls.length).toBeGreaterThan(before)
  })

  it('【关键】更新：请求体**不带 name**（项目名即部署目录，不可改）', async () => {
    const ctl = mountList()
    await settle()

    await ctl.update(3, form({ name: 'blog', domain: 'b.com' }) as never)
    const body = api.updateProject.mock.calls[0][1] as Record<string, unknown>
    expect(body).not.toHaveProperty('name')
    expect(body).toHaveProperty('domain', 'b.com')
    expect(api.updateProject.mock.calls[0][0]).toBe(3)
  })

  it('【关键】空串归一为 null：domain/run_command/health_check_path/source_url', async () => {
    const ctl = mountList()
    await settle()

    await ctl.create(
      form({ source_url: '', run_command: '', domain: '', health_check_path: '' }) as never,
    )
    expect(api.createProject).toHaveBeenCalledWith(
      expect.objectContaining({
        source_url: null,
        run_command: null,
        domain: null,
        health_check_path: null,
      }),
    )
  })

  it('失败：返回 false、toast 错误、提交态复位、不刷新', async () => {
    api.createProject.mockRejectedValue(new Error('同名项目已存在'))
    const ctl = mountList()
    await settle()
    const before = api.listProjects.mock.calls.length

    const ok = await ctl.create(form() as never)
    expect(ok).toBe(false)
    expect(ctl.submitting.value).toBe(false)
    expect(toast.error).toHaveBeenCalled()
    expect(api.listProjects.mock.calls.length).toBe(before)
  })

  it('更新失败同样返回 false（调用方据此不关弹窗）', async () => {
    api.updateProject.mockRejectedValue(new Error('运行中改启动命令 409'))
    const ctl = mountList()
    await settle()
    expect(await ctl.update(3, form() as never)).toBe(false)
  })
})

describe('useProjectList · 运行时候选', () => {
  it('【关键】同 kind 只留首个版本', async () => {
    api.listRuntimes.mockResolvedValue({
      data: [
        {
          id: 7,
          kind: 'node',
          version: '20',
          status: 'installed',
          install_path: '',
          is_default: true,
          created_at: '',
          updated_at: '',
        },
        {
          id: 8,
          kind: 'node',
          version: '18',
          status: 'installed',
          install_path: '',
          is_default: false,
          created_at: '',
          updated_at: '',
        },
        {
          id: 9,
          kind: 'php',
          version: '8.2',
          status: 'installed',
          install_path: '',
          is_default: false,
          created_at: '',
          updated_at: '',
        },
      ],
    })
    const ctl = mountList()
    await settle()
    expect(ctl.runtimeSelectOptions.value).toEqual([
      { label: 'node 20', value: 7 },
      { label: 'php 8.2', value: 9 },
    ])
  })
})
