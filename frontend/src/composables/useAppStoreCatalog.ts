import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import { getErrorDetail } from '@/utils/error'
import { useLocalizedDesc } from '@/composables/useLocalizedDesc'
import {
  listPackages,
  listInstalledApps,
  listWasmBuiltins,
  uninstallApp,
  upgradeApp,
  getAppLogs,
  importPackage,
  nativeAppStart,
  nativeAppStop,
  nativeAppRestart,
  nativeAppStatus,
  type AppMetadata,
  type InstalledApp,
} from '@/api/appStore'

/**
 * 应用商店数据层（Doc/23 §3.3 T2-4 视图拆分第一步）。
 *
 * 原 AppStoreView 把「列表加载 + 搜索 + 卸装/升级 + 原生生命周期 + 日志/导入」
 * 与模板揉在同一个 1464 行文件里。本 composable 承接全部**非安装弹窗**的数据逻辑：
 *
 * - 三类列表（商店包 / 已安装 / WASM 内置工具）的加载态与数据；
 * - 搜索过滤与推荐位、已安装 key 集合三个派生视图（供卡片网格消费）；
 * - 行级操作（卸载 / 升级 / 原生启停）与日志弹窗、本地包导入。
 *
 * 安装弹窗的状态机因体量单独落在 `useAppStoreInstall`（弹窗组件内部使用），
 * 两者唯一的耦合点是「安装成功后刷新已安装列表」——由视图层在 `installed` 事件里回灌。
 * 行为与原实现逐行对齐（含错误文案与静默容错），无契约变更。
 */
export function useAppStoreCatalog() {
  const { t } = useI18n()
  const toast = useFpToast()
  const { confirmAction } = useFpConfirm()
  const { matchesQuery } = useLocalizedDesc()

  const loading = ref(false)
  const loadingInstalled = ref(false)
  const loadingWasm = ref(false)
  const packages = ref<AppMetadata[]>([])
  const installedApps = ref<InstalledApp[]>([])
  const wasmBuiltins = ref<AppMetadata[]>([])
  const search = ref('')

  const logsVisible = ref(false)
  const currentLogs = ref('')

  // T0-4（P0-6）：描述按界面语言取双语字段，搜索同时覆盖中英字段
  const filteredPackages = computed(() =>
    packages.value.filter((p) => matchesQuery(p, search.value)),
  )
  // 推荐安装（后端 recommended 标记）
  const recommendedPackages = computed(() => packages.value.filter((p) => p.recommended))
  // 已安装应用 key 集合（角标/按钮状态）
  const installedKeys = computed(() => new Set(installedApps.value.map((a) => a.package_key)))

  async function loadPackages() {
    loading.value = true
    try {
      const res = await listPackages()
      packages.value = res.data.packages
    } finally {
      loading.value = false
    }
  }

  async function loadInstalled() {
    loadingInstalled.value = true
    try {
      const res = await listInstalledApps()
      installedApps.value = res.data
    } finally {
      loadingInstalled.value = false
    }
  }

  async function loadWasm() {
    loadingWasm.value = true
    try {
      const res = await listWasmBuiltins()
      wasmBuiltins.value = res.data
    } finally {
      loadingWasm.value = false
    }
  }

  function uninstall(row: InstalledApp) {
    confirmAction({
      message: t('appStore.confirmUninstall', { name: row.name }),
      header: t('common.warning'),
      accept: async () => {
        try {
          await uninstallApp(row.id)
          toast.success(t('appStore.uninstallSuccess'))
          loadInstalled()
        } catch (e: unknown) {
          toast.error(e, getErrorDetail(e) || t('appStore.uninstallFailed'))
        }
      },
    })
  }

  async function upgrade(row: InstalledApp) {
    try {
      await upgradeApp(row.id)
      toast.success(t('appStore.upgradeSuccess'))
      loadInstalled()
    } catch (e: unknown) {
      toast.error(e, getErrorDetail(e) || t('appStore.upgradeFailed'))
    }
  }

  async function refreshNativeStatus(row: InstalledApp) {
    try {
      const res = await nativeAppStatus(row.id)
      const target = installedApps.value.find((a) => a.id === row.id)
      if (target) target.status = res.data.status
    } catch {
      /* 状态刷新失败不阻断交互 */
    }
  }

  async function nativeAction(
    row: InstalledApp,
    action: (id: number) => Promise<unknown>,
  ): Promise<void> {
    try {
      await action(row.id)
      toast.success(t('common.success'))
      await refreshNativeStatus(row)
    } catch (e: unknown) {
      toast.error(e, getErrorDetail(e) || t('common.failed'))
    }
  }

  const handleNativeStart = (row: InstalledApp) => nativeAction(row, nativeAppStart)
  const handleNativeStop = (row: InstalledApp) => nativeAction(row, nativeAppStop)
  const handleNativeRestart = (row: InstalledApp) => nativeAction(row, nativeAppRestart)

  async function showLogs(row: InstalledApp) {
    try {
      const res = await getAppLogs(row.id)
      currentLogs.value = res.data.logs || '-'
      logsVisible.value = true
    } catch {
      currentLogs.value = '-'
      logsVisible.value = true
    }
  }

  async function doImport(path: string): Promise<boolean> {
    if (!path) return false
    try {
      await importPackage(path)
      toast.success(t('appStore.importSuccess'))
      loadPackages()
      return true
    } catch (e: unknown) {
      toast.error(e, getErrorDetail(e) || t('appStore.importFailed'))
      return false
    }
  }

  return {
    loading,
    loadingInstalled,
    loadingWasm,
    packages,
    installedApps,
    wasmBuiltins,
    search,
    logsVisible,
    currentLogs,
    filteredPackages,
    recommendedPackages,
    installedKeys,
    loadPackages,
    loadInstalled,
    loadWasm,
    uninstall,
    upgrade,
    refreshNativeStatus,
    handleNativeStart,
    handleNativeStop,
    handleNativeRestart,
    showLogs,
    doImport,
  }
}
