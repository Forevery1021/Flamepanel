import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import { getErrorDetail } from '@/utils/error'
import { detectComposeRisk } from '@/utils/composeRisk'
import { modeLabel } from '@/utils/appStoreFormat'
import {
  getPackage,
  getPackageVersion,
  installApp,
  type AppMetadata,
  type AppVersionInfo,
} from '@/api/appStore'

/**
 * 应用安装弹窗状态机（Doc/23 §3.3 T2-4 视图拆分第一步）。
 *
 * 原 AppStoreView 内联实现，承担三件事：版本/参数加载、风险提示（privileged、
 * docker.sock、生命周期钩子脚本）、提交安装。拆出后由 `AppInstallModal.vue` 独占消费，
 * 视图层只持有组件 ref 并注入待安装的包——数据逻辑可脱离模板单测。
 *
 * 行为与原实现逐行对齐：`open()` 先落基础信息再回查完整包元数据（WASM 内置工具
 * 无包记录，回查失败静默保留入参）；`version` 变更触发 `loadVersion` 重取参数表单。
 */
export function useAppStoreInstall() {
  const { t } = useI18n()
  const toast = useFpToast()

  const visible = ref(false)
  const loading = ref(false)
  const meta = ref<AppMetadata | null>(null)
  const versionInfo = ref<AppVersionInfo | null>(null)
  const securityWarning = ref('')

  const form = reactive<{
    version: string
    mode: string
    name: string
    port?: number
    container_name: string
    confirm_risky: boolean
    acknowledge_scripts: boolean
    values: Record<string, string>
  }>({
    version: '',
    mode: '',
    name: '',
    container_name: '',
    confirm_risky: false,
    acknowledge_scripts: false,
    values: {},
  })

  /** switch / number 型字段与文本字段分表承载：提交时统一回写 `values`（字符串协议） */
  const switchValues = reactive<Record<string, boolean>>({})
  const numberValues = reactive<Record<string, number>>({})

  const versionOptions = computed(() =>
    (meta.value?.versions || []).map((v) => ({ label: v, value: v })),
  )
  const modeOptions = computed(() =>
    (meta.value?.modes || []).map((m) => ({ label: modeLabel(m, t), value: m })),
  )

  // Doc/20 §4.3：当前版本声明的全部钩子脚本（用于安装弹窗确认提示）
  const hookScripts = computed(() => {
    const h = versionInfo.value?.hooks
    if (!h) return []
    return [h.pre_install, h.post_install, h.pre_upgrade, h.post_upgrade, h.pre_uninstall].filter(
      (s): s is string => !!s,
    )
  })

  async function open(pkg: AppMetadata) {
    meta.value = pkg
    form.version = pkg.default_version
    form.mode = pkg.modes[0] ?? 'container'
    form.name = pkg.name
    form.port = undefined
    form.container_name = ''
    form.confirm_risky = false
    form.acknowledge_scripts = false
    form.values = {}
    securityWarning.value = ''
    visible.value = true
    try {
      const res = await getPackage(pkg.key)
      meta.value = res.data
    } catch {
      // builtin wasm tools have no package record
    }
    await loadVersion()
  }

  async function loadVersion() {
    versionInfo.value = null
    if (!meta.value) return
    try {
      const res = await getPackageVersion(meta.value.key, form.version)
      versionInfo.value = res.data
      for (const f of res.data.form_fields ?? []) {
        if (f.default && !(f.env_key in form.values)) {
          form.values[f.env_key] = f.default
        }
        if (f.field_type === 'switch') switchValues[f.env_key] = f.default === 'true'
        if (f.field_type === 'number' || f.field_type === 'port') {
          numberValues[f.env_key] = f.default ? Number(f.default) : 0
        }
      }
      const compose = res.data.compose_template ?? ''
      // 与后端结构化扫描同口径的前置提示（权威判定在后端，未确认即阻断）
      const risk = form.confirm_risky ? null : detectComposeRisk(compose)
      if (risk === 'privileged') securityWarning.value = t('appStore.warnPrivileged')
      else if (risk === 'dockerSock') securityWarning.value = t('appStore.warnDockerSock')
      else if (risk === 'hostAccess') securityWarning.value = t('appStore.warnHostAccess')
    } catch {
      // version info unavailable
    }
  }

  watch(() => form.version, loadVersion)

  /** 提交安装；返回是否成功（成功由弹窗组件向视图层上抛 `installed` 以刷新列表） */
  async function submit(): Promise<boolean> {
    if (!meta.value) return false
    loading.value = true
    try {
      const values: Record<string, string> = { ...form.values }
      for (const key of Object.keys(switchValues)) values[key] = String(switchValues[key])
      for (const key of Object.keys(numberValues)) {
        if (numberValues[key] !== 0) values[key] = String(numberValues[key])
      }
      // 含钩子脚本时必须显式确认执行宿主机脚本
      if (hookScripts.value.length && !form.acknowledge_scripts) {
        toast.error(t('appStore.hooksAck'))
        return false
      }
      await installApp(meta.value.key, {
        package_key: meta.value.key,
        version: form.version,
        mode: form.mode,
        name: form.name,
        port: form.port,
        container_name: form.container_name,
        values,
        confirm_risky: form.confirm_risky,
        acknowledge_scripts: form.acknowledge_scripts,
      })
      toast.success(t('appStore.installSuccess'))
      visible.value = false
      return true
    } catch (e: unknown) {
      toast.error(e, getErrorDetail(e) || t('appStore.installFailed'))
      return false
    } finally {
      loading.value = false
    }
  }

  return {
    visible,
    loading,
    meta,
    versionInfo,
    securityWarning,
    form,
    switchValues,
    numberValues,
    versionOptions,
    modeOptions,
    hookScripts,
    open,
    loadVersion,
    submit,
  }
}
