import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import { getErrorDetail } from '@/utils/error'
import {
  listSources,
  createSource,
  updateSource,
  deleteSource,
  syncSource,
  type AppSource,
} from '@/api/appStore'

/** 创建远程源的提交载荷（表单编辑态在 SourceDialogs 内，避免跨组件 mutate 对象） */
export interface SourceCreatePayload {
  name: string
  source_type: string
  url: string
  public_key: string
}

/**
 * 远程应用源管理（Doc/23 §3.3 T2-4 视图拆分的独立数据层）。
 *
 * 原 AppStoreView 内联实现：列表加载、添加（未签名源二次确认）、启停、同步、
 * 删除、公钥编辑（清空 = 卸载信任锚，须再次确认）。语义与实现逐行对齐，含两处
 * 安全确认分支——它们是 Doc/20 §4.2「未签名第三方源二次确认」的前端落点。
 *
 * **表单编辑态不在此**：弹窗内的输入值属于「编辑会话」，归 `SourceDialogs`
 * 本地 ref 持有，提交时把载荷传进来（`create(payload)` / `savePublicKey(value)`）。
 */
export function useAppStoreSources() {
  const { t } = useI18n()
  const toast = useFpToast()
  const { confirmAction } = useFpConfirm()

  const sources = ref<AppSource[]>([])
  const loading = ref(false)
  const syncingSourceId = ref<number | null>(null)
  const creating = ref(false)

  // ── 公钥编辑（Doc/20 §4.2：签名源降级为未签名源须二次确认） ──
  const savingKey = ref(false)
  const keyDialogSource = ref<AppSource | null>(null)

  async function loadSources() {
    loading.value = true
    try {
      const res = await listSources()
      sources.value = res.data.sources
    } finally {
      loading.value = false
    }
  }

  /**
   * 创建远程源。
   * Doc/20 §4.2：未配置公钥的第三方源默认弹风险确认（前端二次确认 + 后端强制门禁）。
   * 成功返回 true，由调用方决定是否关闭弹窗。
   */
  async function create(payload: SourceCreatePayload): Promise<boolean> {
    if (!payload.name || !payload.url) return false
    if (!payload.public_key.trim()) {
      return await new Promise<boolean>((resolve) => {
        confirmAction({
          message: t('appStore.confirmUnsignedSource', { name: payload.name }),
          header: t('appStore.unsignedSourceTitle'),
          accept: () => {
            submitCreate(payload, true).then(resolve)
          },
          reject: () => resolve(false),
        })
      })
    }
    return submitCreate(payload, false)
  }

  async function submitCreate(
    payload: SourceCreatePayload,
    confirmUnsigned: boolean,
  ): Promise<boolean> {
    creating.value = true
    try {
      await createSource({
        name: payload.name,
        source_type: payload.source_type,
        url: payload.url,
        public_key: payload.public_key || null,
        confirm_unsigned: confirmUnsigned,
      })
      toast.success(t('appStore.sourceCreated'))
      await loadSources()
      return true
    } catch (e: unknown) {
      toast.error(e, getErrorDetail(e) || t('common.failed'))
      return false
    } finally {
      creating.value = false
    }
  }

  /** 同步源索引；成功返回 true（调用方据此刷新商店包列表） */
  async function sync(row: AppSource): Promise<boolean> {
    syncingSourceId.value = row.id
    try {
      const res = await syncSource(row.id)
      toast.success(t('appStore.syncDone', { added: res.data.added, skipped: res.data.skipped }))
      return true
    } catch (e: unknown) {
      toast.error(e, getErrorDetail(e) || t('appStore.syncFailed'))
      return false
    } finally {
      syncingSourceId.value = null
    }
  }

  function toggle(row: AppSource) {
    confirmAction({
      message: row.enabled
        ? t('appStore.confirmDisable', { name: row.name })
        : t('appStore.confirmEnable', { name: row.name }),
      header: t('common.warning'),
      accept: async () => {
        try {
          await updateSource(row.id, { enabled: !row.enabled })
          toast.success(t('common.success'))
          loadSources()
        } catch (e: unknown) {
          toast.error(e, getErrorDetail(e) || t('common.failed'))
        }
      },
    })
  }

  function remove(row: AppSource) {
    confirmAction({
      message: t('appStore.confirmDeleteSource', { name: row.name }),
      header: t('common.warning'),
      accept: async () => {
        try {
          await deleteSource(row.id)
          toast.success(t('appStore.sourceDeleted'))
          loadSources()
        } catch (e: unknown) {
          toast.error(e, getErrorDetail(e) || t('appStore.sourceDeleteFailed'))
        }
      },
    })
  }

  function openKeyDialog(row: AppSource) {
    keyDialogSource.value = row
  }

  /**
   * 保存公钥。清空公钥 = 卸载信任锚，须显式二次确认（Doc/20 §4.2）。
   * 成功返回 true，由调用方决定是否关闭弹窗。
   */
  async function savePublicKey(value: string): Promise<boolean> {
    const row = keyDialogSource.value
    if (!row) return false
    const wasSigned = !!row.public_key?.trim()
    if (wasSigned && !value.trim()) {
      return await new Promise<boolean>((resolve) => {
        confirmAction({
          message: t('appStore.confirmClearPublicKey', { name: row.name }),
          header: t('appStore.unsignedSourceTitle'),
          accept: () => {
            submitPublicKey(row, value, true).then(resolve)
          },
          reject: () => resolve(false),
        })
      })
    }
    return submitPublicKey(row, value, false)
  }

  async function submitPublicKey(
    row: AppSource,
    value: string,
    confirmUnsigned: boolean,
  ): Promise<boolean> {
    savingKey.value = true
    try {
      await updateSource(row.id, {
        public_key: value.trim() || null,
        confirm_unsigned: confirmUnsigned,
      })
      toast.success(t('common.success'))
      await loadSources()
      return true
    } catch (e: unknown) {
      toast.error(e, getErrorDetail(e) || t('common.failed'))
      return false
    } finally {
      savingKey.value = false
    }
  }

  return {
    sources,
    loading,
    syncingSourceId,
    creating,
    savingKey,
    keyDialogSource,
    loadSources,
    create,
    sync,
    toggle,
    remove,
    openKeyDialog,
    savePublicKey,
  }
}
