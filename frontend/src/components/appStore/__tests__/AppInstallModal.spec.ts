import { describe, it, expect, vi, beforeEach } from 'vitest'
import { nextTick } from 'vue'
import { mountFp } from '@/components/ui/__tests__/helpers'
import AppInstallModal from '@/components/appStore/AppInstallModal.vue'

/**
 * T2-4 拆分后的安装弹窗外壳契约。
 *
 * 状态机在 `useAppStoreInstall`（15 例单测），本组件只做两件事：
 * ① 以 `open()` 暴露打开入口（卡片点击 → ref 调用）；② 安装成功后上抛 `installed`
 * （视图层据此刷新已安装列表）。二者正是拆分后新的接线面，必须看住。
 */

const { api } = vi.hoisted(() => ({
  api: {
    getPackage: vi.fn(),
    getPackageVersion: vi.fn(),
    installApp: vi.fn(),
  },
}))

vi.mock('@/api/appStore', () => api)
vi.mock('@/components/ui/FpToast', () => ({
  useFpToast: () => ({ success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() }),
}))

const meta = {
  key: 'gitea',
  name: 'Gitea',
  category: 'tools',
  short_desc_zh: '轻量 Git',
  tags: [],
  format: 'flame',
  modes: ['container'],
  versions: ['1.0.0'],
  default_version: '1.0.0',
  architectures: [],
}

beforeEach(() => {
  vi.clearAllMocks()
  document.body.innerHTML = ''
  api.getPackage.mockResolvedValue({ data: meta })
  api.getPackageVersion.mockResolvedValue({
    data: {
      version: '1.0.0',
      mode: 'container',
      form_fields: [],
      compose_template: '',
      native_scripts: [],
      architectures: [],
    },
  })
  api.installApp.mockResolvedValue({ data: {} })
})

async function settle() {
  await nextTick()
  await nextTick()
  await nextTick()
}

describe('AppInstallModal', () => {
  it('初始不渲染弹窗（visible=false）', async () => {
    mountFp(AppInstallModal as never, { messages: { appStore: { installTitle: '安装应用' } } })
    await settle()
    expect(document.body.querySelector('.p-dialog')).toBeNull()
  })

  it('open() 打开弹窗并渲染包名（视图层经 ref 调用该入口）', async () => {
    const w = mountFp(AppInstallModal as never, {
      messages: { appStore: { installTitle: '安装应用', fldVersion: '版本', fldMode: '模式', fldName: '名称', fldPort: '端口' } },
      global: { directives: { permission: () => {} } },
    })
    const vm = w.vm as unknown as { open: (pkg: unknown) => Promise<void> }
    await vm.open(meta)
    await settle()
    expect(document.body.textContent).toContain('Gitea')
    expect(api.getPackage).toHaveBeenCalledWith('gitea')
  })

  it('安装成功 → 上抛 installed（视图层据此刷新已安装列表）', async () => {
    const w = mountFp(AppInstallModal as never, {
      messages: {
        appStore: {
          installTitle: '安装应用',
          install: '安装',
          fldVersion: '版本',
          fldMode: '模式',
          fldName: '名称',
          fldPort: '端口',
          confirmRisky: '我已确认风险',
        },
      },
      global: { directives: { permission: () => {} } },
    })
    const vm = w.vm as unknown as { open: (pkg: unknown) => Promise<void> }
    await vm.open(meta)
    await settle()

    const installBtn = Array.from(document.body.querySelectorAll('button')).find(
      (b) => b.textContent?.trim() === '安装',
    )
    installBtn?.click()
    await settle()
    await settle()

    expect(api.installApp).toHaveBeenCalledWith('gitea', expect.objectContaining({ version: '1.0.0' }))
    expect(w.emitted('installed')).toBeTruthy()
  })
})
