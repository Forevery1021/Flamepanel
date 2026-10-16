import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent, nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import { createTestI18n } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 拆分的数据层单测（`AppStoreView` → `useAppStoreInstall`）。
 *
 * 安装弹窗状态机原先内联在 1464 行视图里，是「装什么、怎么装」的核心路径：
 * 版本参数表单装配（switch/number 分表）、风险提示（privileged / docker.sock）、
 * 钩子脚本强制确认、提交载荷组装。这些此前全部无单测覆盖。
 */

const { api, toast } = vi.hoisted(() => ({
  api: {
    getPackage: vi.fn(),
    getPackageVersion: vi.fn(),
    installApp: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
}))

vi.mock('@/api/appStore', () => api)
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))

import { useAppStoreInstall } from '@/composables/useAppStoreInstall'

const messages = {
  appStore: {
    warnPrivileged: '该应用使用 privileged 模式',
    warnDockerSock: '该应用挂载了 Docker Socket',
    hooksAck: '请确认执行宿主机脚本',
    installSuccess: '安装成功',
    installFailed: '安装失败',
    modeContainer: '容器',
    modeNative: '原生',
  },
}

const meta = {
  key: 'gitea',
  name: 'Gitea',
  category: 'tools',
  short_desc_zh: '轻量 Git',
  tags: [],
  format: 'flame',
  modes: ['container', 'native'],
  versions: ['1.0.0', '1.1.0'],
  default_version: '1.0.0',
  architectures: [],
}

const versionInfo = (over: Record<string, unknown> = {}) => ({
  version: '1.0.0',
  mode: 'container',
  form_fields: [],
  compose_template: '',
  native_scripts: [],
  architectures: [],
  ...over,
})

function makeHost() {
  return defineComponent({
    setup() {
      return { install: useAppStoreInstall() }
    },
    template: '<div/>',
  })
}

function mountInstall() {
  const w = mount(makeHost(), { global: { plugins: [createTestI18n(messages)] } })
  const install = (w.vm as unknown as { install: ReturnType<typeof useAppStoreInstall> }).install
  return { w, install }
}

beforeEach(() => {
  vi.clearAllMocks()
  api.getPackage.mockResolvedValue({ data: meta })
  api.getPackageVersion.mockResolvedValue({ data: versionInfo() })
  api.installApp.mockResolvedValue({ data: {} })
})

describe('useAppStoreInstall · open 初始化', () => {
  it('以卡片元数据开弹窗：版本/模式/名称取默认值，风险与确认项复位', async () => {
    const { install } = mountInstall()
    await install.open(meta as never)

    expect(install.visible.value).toBe(true)
    expect(install.form.version).toBe('1.0.0')
    expect(install.form.mode).toBe('container')
    expect(install.form.name).toBe('Gitea')
    expect(install.form.confirm_risky).toBe(false)
    expect(install.form.acknowledge_scripts).toBe(false)
    expect(install.form.values).toEqual({})
    expect(install.securityWarning.value).toBe('')
  })

  it('回查完整包元数据（卡片数据可能精简）；WASM 无包记录时静默保留入参', async () => {
    api.getPackage.mockResolvedValue({ data: { ...meta, name: 'Gitea 全量' } })
    const { install } = mountInstall()
    await install.open(meta as never)
    expect(install.meta.value?.name).toBe('Gitea 全量')

    api.getPackage.mockRejectedValue(new Error('404'))
    const { install: i2 } = mountInstall()
    await i2.open({ ...meta, key: 'wasm-tool' } as never)
    expect(i2.meta.value?.key).toBe('wasm-tool')
    expect(i2.visible.value).toBe(true)
  })

  it('版本参数表单装配：默认值入 values；switch/number 分表（提交时再合并回 values）', async () => {
    api.getPackageVersion.mockResolvedValue({
      data: versionInfo({
        form_fields: [
          { env_key: 'PORT', field_type: 'port', default: '3000', required: true, options: [] },
          { env_key: 'DEBUG', field_type: 'switch', default: 'true', required: false, options: [] },
          { env_key: 'TZ', field_type: 'text', default: 'Asia/Shanghai', required: false, options: [] },
          { env_key: 'EMPTY', field_type: 'text', required: false, options: [] },
        ],
      }),
    })
    const { install } = mountInstall()
    await install.open(meta as never)

    expect(install.form.values.TZ).toBe('Asia/Shanghai')
    expect(install.form.values.EMPTY).toBeUndefined()
    expect(install.switchValues.DEBUG).toBe(true)
    expect(install.numberValues.PORT).toBe(3000)
  })

  it('风险提示：privileged 优先于 docker.sock（两者都命中时展示更严重的一条）', async () => {
    api.getPackageVersion.mockResolvedValue({
      data: versionInfo({ compose_template: 'services:\n  x:\n    privileged: true\n    volumes: /var/run/docker.sock' }),
    })
    const { install } = mountInstall()
    await install.open(meta as never)
    expect(install.securityWarning.value).toBe('该应用使用 privileged 模式')
  })

  it('docker.sock 单独命中时展示对应提示', async () => {
    api.getPackageVersion.mockResolvedValue({
      data: versionInfo({ compose_template: 'volumes:\n  - /var/run/docker.sock:/var/run/docker.sock' }),
    })
    const { install } = mountInstall()
    await install.open(meta as never)
    expect(install.securityWarning.value).toBe('该应用挂载了 Docker Socket')
  })

  it('版本信息取不到不抛错（弹窗仍可用，只是没有参数表单）', async () => {
    api.getPackageVersion.mockRejectedValue(new Error('404'))
    const { install } = mountInstall()
    await install.open(meta as never)
    expect(install.versionInfo.value).toBeNull()
    expect(install.visible.value).toBe(true)
  })
})

describe('useAppStoreInstall · 钩子脚本确认（Doc/20 §4.3）', () => {
  const hookInfo = versionInfo({
    hooks: { pre_install: 'setup.sh', post_install: 'init.sh', pre_upgrade: 'up.sh' },
  })

  it('声明钩子脚本时全部列出（含升级/卸载钩子）', async () => {
    api.getPackageVersion.mockResolvedValue({ data: hookInfo })
    const { install } = mountInstall()
    await install.open(meta as never)
    expect(install.hookScripts.value).toEqual(['setup.sh', 'init.sh', 'up.sh'])
  })

  it('未勾选确认 → **拒绝提交**（宿主机脚本必须显式接受）', async () => {
    api.getPackageVersion.mockResolvedValue({ data: hookInfo })
    const { install } = mountInstall()
    await install.open(meta as never)

    await expect(install.submit()).resolves.toBe(false)
    expect(api.installApp).not.toHaveBeenCalled()
    expect(toast.error).toHaveBeenCalledWith('请确认执行宿主机脚本')
  })

  it('勾选确认后提交，acknowledge_scripts=true 随载荷上送', async () => {
    api.getPackageVersion.mockResolvedValue({ data: hookInfo })
    const { install } = mountInstall()
    await install.open(meta as never)
    install.form.acknowledge_scripts = true

    await expect(install.submit()).resolves.toBe(true)
    expect(api.installApp).toHaveBeenCalledWith(
      'gitea',
      expect.objectContaining({ acknowledge_scripts: true }),
    )
    expect(install.visible.value).toBe(false)
  })

  it('无钩子脚本时不受该门禁约束', async () => {
    const { install } = mountInstall()
    await install.open(meta as never)
    await expect(install.submit()).resolves.toBe(true)
  })
})

describe('useAppStoreInstall · 提交载荷', () => {
  it('switch/number 分表合并回 values（number 为 0 视为未填，不上送）', async () => {
    api.getPackageVersion.mockResolvedValue({
      data: versionInfo({
        form_fields: [
          { env_key: 'DEBUG', field_type: 'switch', default: 'true', required: false, options: [] },
          { env_key: 'PORT', field_type: 'port', default: '3000', required: true, options: [] },
          { env_key: 'ZERO', field_type: 'number', default: '', required: false, options: [] },
        ],
      }),
    })
    const { install } = mountInstall()
    await install.open(meta as never)
    install.form.confirm_risky = true

    await install.submit()
    const [, body] = api.installApp.mock.calls[0]
    expect(body.values.DEBUG).toBe('true')
    expect(body.values.PORT).toBe('3000')
    expect(body.values.ZERO).toBeUndefined()
    expect(body.confirm_risky).toBe(true)
  })

  it('提交失败返回 false 且弹窗保持打开（用户可修正参数重试）', async () => {
    api.installApp.mockRejectedValue({ response: { data: { detail: '端口被占用' } } })
    const { install } = mountInstall()
    await install.open(meta as never)
    await expect(install.submit()).resolves.toBe(false)
    expect(install.visible.value).toBe(true)
    expect(install.loading.value).toBe(false)
  })

  it('未打开弹窗（meta 为空）直接拒绝提交', async () => {
    const { install } = mountInstall()
    await expect(install.submit()).resolves.toBe(false)
    expect(api.installApp).not.toHaveBeenCalled()
  })

  it('切换版本触发参数重载（表单与勾选态跟随新版本）', async () => {
    const { install } = mountInstall()
    await install.open(meta as never)
    expect(api.getPackageVersion).toHaveBeenLastCalledWith('gitea', '1.0.0')

    install.form.version = '1.1.0'
    await nextTick()
    await new Promise((r) => setTimeout(r, 10))
    expect(api.getPackageVersion).toHaveBeenLastCalledWith('gitea', '1.1.0')
  })

  it('模式下拉文案取 i18n（wasm 为技术名词不翻译）', async () => {
    api.getPackage.mockResolvedValue({ data: { ...meta, modes: ['container', 'native'] } })
    const { install } = mountInstall()
    await install.open(meta as never)
    expect(install.modeOptions.value.map((o) => o.label)).toEqual(['容器', '原生'])
  })
})
