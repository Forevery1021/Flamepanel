import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent, nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import { createTestI18n } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 拆分的数据层单测（`AppStoreView` → `useAppStoreCatalog`）。
 *
 * 这层逻辑原先是「巨型视图内的内联函数」——既在 `src/views/**`（覆盖率排除区），
 * 又没人能脱离整页挂载去验证它。拆出后本 spec 直接看住三件事：
 * ① 列表加载与错误态收敛；② 搜索过滤与推荐位/已安装集合三个派生视图；
 * ③ 行级操作（卸载确认、升级、日志、原生启停）的请求与容错分支。
 */

// vi.mock 工厂被提升到文件顶部，故共享桩必须用 vi.hoisted 一并提升
const { api, toast, confirmState } = vi.hoisted(() => ({
  api: {
    listPackages: vi.fn(),
    listInstalledApps: vi.fn(),
    listWasmBuiltins: vi.fn(),
    uninstallApp: vi.fn(),
    upgradeApp: vi.fn(),
    getAppLogs: vi.fn(),
    importPackage: vi.fn(),
    nativeAppStart: vi.fn(),
    nativeAppStop: vi.fn(),
    nativeAppRestart: vi.fn(),
    nativeAppStatus: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
  /** 捕获确认回调（不自动执行），用例决定 accept/reject 何时触发 */
  confirmState: { last: null as { accept?: () => void; message: string } | null },
}))

vi.mock('@/api/appStore', () => api)
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))
vi.mock('@/components/ui/FpConfirm', () => ({
  useFpConfirm: () => ({
    confirmAction: (opts: { accept?: () => void; message: string }) => {
      confirmState.last = opts
    },
  }),
}))

import { useAppStoreCatalog } from '@/composables/useAppStoreCatalog'

const messages = {
  appStore: {
    confirmUninstall: '确定卸载 {name}？',
    uninstallSuccess: '卸载成功',
    uninstallFailed: '卸载失败',
    upgradeSuccess: '升级成功',
    upgradeFailed: '升级失败',
    importSuccess: '导入成功',
    importFailed: '导入失败',
  },
  common: { warning: '警告', success: '成功', failed: '操作失败' },
}

/** 宿主组件：把 composable 暴露到 setupState，用例直接读写 */
function makeHost() {
  return defineComponent({
    setup() {
      return { catalog: useAppStoreCatalog() }
    },
    template: '<div/>',
  })
}

function mountCatalog() {
  const w = mount(makeHost(), { global: { plugins: [createTestI18n(messages)] } })
  const catalog = (w.vm as unknown as { catalog: ReturnType<typeof useAppStoreCatalog> }).catalog
  return { w, catalog }
}

const pkg = (key: string, extra: Record<string, unknown> = {}) => ({
  key,
  name: key.toUpperCase(),
  category: 'tools',
  short_desc_zh: `${key} 中文`,
  short_desc_en: `${key} en`,
  tags: [],
  format: 'flame',
  modes: ['container'],
  versions: ['1.0.0'],
  default_version: '1.0.0',
  architectures: [],
  ...extra,
})

beforeEach(() => {
  vi.clearAllMocks()
  confirmState.last = null
  api.listPackages.mockResolvedValue({ data: { packages: [], total: 0 } })
  api.listInstalledApps.mockResolvedValue({ data: [] })
  api.listWasmBuiltins.mockResolvedValue({ data: [] })
  api.nativeAppStatus.mockResolvedValue({ data: { id: 1, status: 'running' } })
})

describe('useAppStoreCatalog · 列表加载', () => {
  it('loadPackages 落数据并复位 loading（含异常路径）', async () => {
    api.listPackages.mockResolvedValue({ data: { packages: [pkg('gitea')], total: 1 } })
    const { catalog } = mountCatalog()
    expect(catalog.loading.value).toBe(false)

    await catalog.loadPackages()
    expect(catalog.packages.value).toHaveLength(1)
    expect(catalog.loading.value).toBe(false)
    expect(api.listPackages).toHaveBeenCalledTimes(1)
  })

  it('请求失败时 loading 也必须复位（finally 语义，避免页面卡死转圈）', async () => {
    api.listPackages.mockRejectedValue(new Error('boom'))
    const { catalog } = mountCatalog()
    await expect(catalog.loadPackages()).rejects.toThrow('boom')
    expect(catalog.loading.value).toBe(false)
  })

  it('loadInstalled / loadWasm 各自独立加载', async () => {
    api.listInstalledApps.mockResolvedValue({
      data: [{ id: 1, name: 'Nginx', mode: 'container', status: 'running', package_key: 'nginx' }],
    })
    api.listWasmBuiltins.mockResolvedValue({ data: [pkg('wasm-tool')] })
    const { catalog } = mountCatalog()

    await catalog.loadInstalled()
    await catalog.loadWasm()
    expect(catalog.installedApps.value).toHaveLength(1)
    expect(catalog.wasmBuiltins.value).toHaveLength(1)
  })
})

describe('useAppStoreCatalog · 派生视图', () => {
  it('搜索同时覆盖中英字段与包名（T0-4 语义，拆分后不回归）', async () => {
    api.listPackages.mockResolvedValue({
      data: { packages: [pkg('gitea'), pkg('redis')], total: 2 },
    })
    const { catalog } = mountCatalog()
    await catalog.loadPackages()

    catalog.search.value = 'gitea'
    await nextTick()
    expect(catalog.filteredPackages.value.map((p) => p.key)).toEqual(['gitea'])

    catalog.search.value = 'redis en'
    await nextTick()
    expect(catalog.filteredPackages.value.map((p) => p.key)).toEqual(['redis'])

    catalog.search.value = ''
    await nextTick()
    expect(catalog.filteredPackages.value).toHaveLength(2)
  })

  it('推荐位只取 recommended 标记的包', async () => {
    api.listPackages.mockResolvedValue({
      data: { packages: [pkg('a', { recommended: true }), pkg('b')], total: 2 },
    })
    const { catalog } = mountCatalog()
    await catalog.loadPackages()
    expect(catalog.recommendedPackages.value.map((p) => p.key)).toEqual(['a'])
  })

  it('installedKeys 汇总已安装包 key（卡片角标据此判定「已安装」）', async () => {
    api.listInstalledApps.mockResolvedValue({
      data: [
        { id: 1, package_key: 'gitea' },
        { id: 2, package_key: 'redis' },
      ],
    })
    const { catalog } = mountCatalog()
    await catalog.loadInstalled()
    expect(catalog.installedKeys.value.has('gitea')).toBe(true)
    expect(catalog.installedKeys.value.has('mysql')).toBe(false)
  })
})

describe('useAppStoreCatalog · 行级操作', () => {
  const row = { id: 7, name: 'Gitea', mode: 'container', status: 'running', package_key: 'gitea' }

  it('卸载先弹确认；确认后调接口、提示成功并刷新列表', async () => {
    const { catalog } = mountCatalog()
    catalog.uninstall(row as never)
    expect(confirmState.last?.message).toContain('Gitea')
    expect(api.uninstallApp).not.toHaveBeenCalled()

    confirmState.last?.accept?.()
    await new Promise((r) => setTimeout(r, 10))
    expect(api.uninstallApp).toHaveBeenCalledWith(7)
    expect(toast.success).toHaveBeenCalledWith('卸载成功')
    expect(api.listInstalledApps).toHaveBeenCalled()
  })

  it('卸载失败：错误上抛到 toast（不阻断界面）', async () => {
    api.uninstallApp.mockRejectedValue({ response: { data: { detail: '容器占用中' } } })
    const { catalog } = mountCatalog()
    catalog.uninstall(row as never)
    confirmState.last?.accept?.()
    await new Promise((r) => setTimeout(r, 10))
    expect(toast.error).toHaveBeenCalled()
  })

  it('升级成功提示 + 刷新', async () => {
    api.upgradeApp.mockResolvedValue({ data: {} })
    const { catalog } = mountCatalog()
    await catalog.upgrade(row as never)
    expect(api.upgradeApp).toHaveBeenCalledWith(7)
    expect(toast.success).toHaveBeenCalledWith('升级成功')
  })

  it('日志弹窗：成功取日志；失败也给空态而不是静默', async () => {
    api.getAppLogs.mockResolvedValueOnce({ data: { logs: 'line1\nline2' } })
    const { catalog } = mountCatalog()
    await catalog.showLogs(row as never)
    expect(catalog.logsVisible.value).toBe(true)
    expect(catalog.currentLogs.value).toContain('line1')

    api.getAppLogs.mockRejectedValueOnce(new Error('down'))
    catalog.logsVisible.value = false
    await catalog.showLogs(row as never)
    expect(catalog.logsVisible.value).toBe(true)
    expect(catalog.currentLogs.value).toBe('-')
  })

  it('原生启停：动作成功后回读状态并写回列表行（按钮禁用态据此翻转）', async () => {
    const app = { ...row, mode: 'native', status: 'stopped' }
    api.listInstalledApps.mockResolvedValue({ data: [app] })
    api.nativeAppStart.mockResolvedValue({ data: {} })
    const { catalog } = mountCatalog()
    await catalog.loadInstalled()

    await catalog.handleNativeStart(catalog.installedApps.value[0])
    expect(api.nativeAppStart).toHaveBeenCalledWith(7)
    expect(api.nativeAppStatus).toHaveBeenCalledWith(7)
    expect(catalog.installedApps.value[0].status).toBe('running')
  })

  it('状态回读失败不阻断交互（静默容错，保持已有状态）', async () => {
    const app = { ...row, mode: 'native', status: 'stopped' }
    api.listInstalledApps.mockResolvedValue({ data: [app] })
    api.nativeAppStop.mockResolvedValue({ data: {} })
    api.nativeAppStatus.mockRejectedValue(new Error('timeout'))
    const { catalog } = mountCatalog()
    await catalog.loadInstalled()

    await catalog.handleNativeStop(catalog.installedApps.value[0])
    expect(toast.success).toHaveBeenCalled()
    expect(catalog.installedApps.value[0].status).toBe('stopped')
  })

  it('导入本地包：成功返回 true 并刷新商店列表', async () => {
    api.importPackage.mockResolvedValue({ data: {} })
    const { catalog } = mountCatalog()
    await expect(catalog.doImport('/opt/apps/x')).resolves.toBe(true)
    expect(api.importPackage).toHaveBeenCalledWith('/opt/apps/x')
    expect(api.listPackages).toHaveBeenCalled()
  })

  it('导入失败返回 false（调用方据此保持弹窗打开）；空路径不发请求', async () => {
    const { catalog } = mountCatalog()
    await expect(catalog.doImport('')).resolves.toBe(false)
    expect(api.importPackage).not.toHaveBeenCalled()

    api.importPackage.mockRejectedValueOnce(new Error('bad path'))
    await expect(catalog.doImport('/nope')).resolves.toBe(false)
    expect(toast.error).toHaveBeenCalled()
  })
})
