import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第三站的数据层单测（`DockerView` → `useDockerCompose`）。
 *
 * 两条保留的原语义：① 部署前的前置校验（名称或 YAML 为空 → warning 且**不发请求**）；
 * ② up/down **不刷新项目表**——它们是长操作，刷新会读到中间态，原实现刻意如此。
 * 第二条容易被「顺手补个 refresh」改掉，故有用例把「不刷新」这一事实写成断言。
 */

const { api, toast } = vi.hoisted(() => ({
  api: {
    listComposeProjects: vi.fn(),
    composeDeploy: vi.fn(),
    composeUp: vi.fn(),
    composeDown: vi.fn(),
    composeConvertRun: vi.fn(),
    containerToCompose: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
}))

vi.mock('@/api/docker', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/docker')>()
  return { ...actual, ...api }
})
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))

import { useDockerCompose, DEFAULT_COMPOSE_YAML } from '@/composables/useDockerCompose'

const messages = {
  common: { success: '操作成功', failed: '操作失败', required: '此项为必填项' },
}

function mountCtl() {
  const Host = defineComponent({
    setup() {
      return { ctl: useDockerCompose() }
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
  return (w.vm as unknown as { ctl: ReturnType<typeof useDockerCompose> }).ctl
}

async function settle() {
  await new Promise((r) => setTimeout(r, 20))
}

beforeEach(() => {
  vi.clearAllMocks()
  api.listComposeProjects.mockResolvedValue({
    data: [{ name: 'myapp', status: 'running(1)', config_files: '/opt/myapp/compose.yaml' }],
  })
  api.composeDeploy.mockResolvedValue({ data: null })
  api.composeUp.mockResolvedValue({ data: null })
  api.composeDown.mockResolvedValue({ data: null })
})

describe('useDockerCompose', () => {
  it('默认 YAML 初值是一段可直接用的配置（不是空串）', () => {
    expect(DEFAULT_COMPOSE_YAML).toContain('services:')
    expect(DEFAULT_COMPOSE_YAML).toContain('image: nginx:alpine')
  })

  it('惰性启用：初始不发请求，置 true 后加载项目列表', async () => {
    const ctl = mountCtl()
    await settle()
    expect(api.listComposeProjects).not.toHaveBeenCalled()

    ctl.enabled.value = true
    await settle()
    expect(api.listComposeProjects).toHaveBeenCalledTimes(1)
    expect(ctl.projects.value).toHaveLength(1)
  })

  it('部署前校验：名称为空 → warning 且不发请求', async () => {
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()

    const ok = await ctl.deploy({ name: '', yaml: DEFAULT_COMPOSE_YAML })
    expect(ok).toBe(false)
    expect(toast.warning).toHaveBeenCalledWith('此项为必填项')
    expect(api.composeDeploy).not.toHaveBeenCalled()
  })

  it('部署前校验：YAML 为空 → warning 且不发请求', async () => {
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()

    const ok = await ctl.deploy({ name: 'myapp', yaml: '' })
    expect(ok).toBe(false)
    expect(api.composeDeploy).not.toHaveBeenCalled()
  })

  it('部署成功：按 (名称, YAML) 上报并刷新项目表', async () => {
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()
    const before = api.listComposeProjects.mock.calls.length

    const ok = await ctl.deploy({ name: 'myapp', yaml: 'services: {}' })
    expect(ok).toBe(true)
    expect(api.composeDeploy).toHaveBeenCalledWith('myapp', 'services: {}')
    expect(api.listComposeProjects.mock.calls.length).toBe(before + 1)
  })

  it('部署失败：返回 false 且提交态复位', async () => {
    api.composeDeploy.mockRejectedValue(new Error('invalid yaml'))
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()

    const ok = await ctl.deploy({ name: 'myapp', yaml: 'bad' })
    expect(ok).toBe(false)
    expect(ctl.submitting.value).toBe(false)
  })

  it('up / down：只调动作接口、**不刷新项目表**（长操作，刷新会读到中间态）', async () => {
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()
    const before = api.listComposeProjects.mock.calls.length

    expect(await ctl.up('myapp')).toBe(true)
    expect(await ctl.down('myapp')).toBe(true)

    expect(api.composeUp).toHaveBeenCalledWith('myapp')
    expect(api.composeDown).toHaveBeenCalledWith('myapp')
    expect(api.listComposeProjects.mock.calls.length).toBe(before)
  })

  it('up 失败：返回 false 且 toast 错误', async () => {
    api.composeUp.mockRejectedValue(new Error('no such project'))
    const ctl = mountCtl()
    ctl.enabled.value = true
    await settle()

    expect(await ctl.up('ghost')).toBe(false)
    expect(toast.error).toHaveBeenCalled()
  })
})

describe('useDockerCompose · 转换（M-D）', () => {
  const conversion = {
    project_name: 'web',
    service_name: 'web',
    compose_yaml: 'services: {}\n',
    warnings: [],
  }

  it('convertRun 返回转换结果，且不触发部署', async () => {
    api.composeConvertRun.mockResolvedValue({ data: conversion })
    const ctl = mountCtl()
    await expect(ctl.convertRun('docker run nginx')).resolves.toEqual(conversion)
    expect(api.composeConvertRun).toHaveBeenCalledWith('docker run nginx')
    expect(api.composeDeploy).not.toHaveBeenCalled()
    expect(ctl.converting.value).toBe(false)
  })

  it('转换失败：toast 报错并返回 null', async () => {
    api.containerToCompose.mockRejectedValue(new Error('boom'))
    const ctl = mountCtl()
    await expect(ctl.exportContainer('abc')).resolves.toBeNull()
    expect(api.containerToCompose).toHaveBeenCalledWith('abc')
    expect(toast.error).toHaveBeenCalled()
    expect(ctl.converting.value).toBe(false)
  })
})
