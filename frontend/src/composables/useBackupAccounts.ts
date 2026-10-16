import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import { useApiQuery } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import {
  listBackupAccounts,
  createBackupAccount,
  updateBackupAccount,
  deleteBackupAccount,
  testBackupAccount,
  listRemoteObjects,
} from '@/api/backupAccounts'
import type { BackupAccountDto, RemoteObjectDto } from '@/api/generated'
import { accountSelectOptions } from '@/utils/backupFormat'

/** 账号表单载荷（编辑态由弹窗组件持有，提交时按此形状上抛） */
export interface BackupAccountFormPayload {
  name: string
  account_type: string
  /** 编辑时留空 = 不修改连接配置（后端不回显敏感 config_json） */
  config_json: string
  enabled: boolean
}

/**
 * 备份账号数据层（Doc/23 §3.3 T2-4 视图拆分第四站）。
 *
 * 两个反直觉的原语义在此固化，拆分不得改：
 *
 * 1. **编辑时 `config_json` 留空 = 不修改**：后端从不回显敏感配置，故编辑弹窗
 *    **不回填**该字段；提交时 `trim() || null` 把空串转成 `null`（协议里 null 才
 *    是「保持原值」），非空串才是新配置。若在这里「顺手」把空串原样上报，
 *    等于每次改名都把连接配置清空。
 * 2. **开关账号必须先确认**：停用会让关联任务直接失败，故 `toggle` 走二次确认，
 *    且确认后失败也刷新（原实现如此）——账号状态可能已被后端改动，
 *    让界面回到真值比保留一个乐观态更安全。
 *
 * 另外云端恢复所需的「远端对象」也归本层：它按账号寻址，换账号要重拉。
 */
export function useBackupAccounts() {
  const { t } = useI18n()
  const toast = useFpToast()
  const { confirmAction } = useFpConfirm()

  const accountsQuery = useApiQuery<BackupAccountDto[]>(
    () => queryKeys.backupAccounts.list(),
    async () => {
      const res = await listBackupAccounts()
      return { data: res.data }
    },
  )
  const accounts = computed<BackupAccountDto[]>(() => accountsQuery.data.value ?? [])
  const loading = accountsQuery.loading
  function fetch() {
    return accountsQuery.refresh()
  }

  /** 目标账号下拉（只列启用账号；任务弹窗与云端恢复共用） */
  const selectOptions = computed(() => accountSelectOptions(accounts.value))

  const testingId = ref<number | null>(null)
  const saving = ref(false)
  const editingId = ref(0)

  /**
   * 进入编辑态：记录目标 id（`0` 为新建）。
   *
   * 编辑目标由本层持有而**不是**由弹窗回报——弹窗只管编辑，`save()` 必须知道
   * 「这次提交是新建还是改哪一行」，让它自己存一份 id 就是把同一条事实写两遍。
   */
  function beginEdit(row?: BackupAccountDto) {
    editingId.value = row?.id ?? 0
  }

  /**
   * 保存（新建或编辑）；成功返回 true（调用方据此关弹窗）。
   * 编辑分支的空配置转 `null`——见文件头语义 1。
   */
  async function save(payload: BackupAccountFormPayload): Promise<boolean> {
    saving.value = true
    try {
      if (editingId.value) {
        await updateBackupAccount(editingId.value, {
          name: payload.name,
          account_type: payload.account_type,
          config_json: payload.config_json.trim() || null,
          enabled: payload.enabled,
        })
      } else {
        await createBackupAccount({
          name: payload.name,
          account_type: payload.account_type,
          config_json: payload.config_json.trim(),
          enabled: payload.enabled,
        })
      }
      toast.success(t('common.success'))
      void accountsQuery.refresh()
      return true
    } catch (err) {
      toast.error(err, t('common.failed'))
      return false
    } finally {
      saving.value = false
    }
  }

  /** 启用 / 停用（二次确认；无论成败都刷新，见文件头语义 2） */
  function toggle(row: BackupAccountDto, enabled: boolean) {
    confirmAction({
      message: enabled
        ? t('backup.accountEnableConfirm', { name: row.name })
        : t('backup.accountDisableConfirm', { name: row.name }),
      header: t('common.confirm'),
      accept: async () => {
        try {
          await updateBackupAccount(row.id, { enabled })
          toast.success(t('common.success'))
          void accountsQuery.refresh()
        } catch {
          toast.error(t('common.failed'))
          void accountsQuery.refresh()
        }
      },
    })
  }

  /** 连通性测试：结果取自响应体的 `last_test_status`（接口失败才走 catch） */
  async function test(row: BackupAccountDto) {
    testingId.value = row.id
    try {
      const res = await testBackupAccount(row.id)
      if (res.data.last_test_status === 'success') {
        toast.success(t('backup.testSuccess'))
      } else {
        toast.error(t('backup.testFailed'))
      }
      void accountsQuery.refresh()
    } catch (err) {
      toast.error(err, t('common.failed'))
    } finally {
      testingId.value = null
    }
  }

  function remove(row: BackupAccountDto) {
    confirmAction({
      message: t('backup.accountDeleteConfirm', { name: row.name }),
      header: t('common.confirm'),
      acceptLabel: t('common.delete'),
      rejectLabel: t('common.cancel'),
      accept: async () => {
        try {
          await deleteBackupAccount(row.id)
          toast.success(t('common.success'))
          void accountsQuery.refresh()
        } catch (err) {
          toast.error(err, t('common.failed'))
        }
      },
    })
  }

  /* ── 云端恢复：远端对象按账号寻址 ── */

  const objects = ref<RemoteObjectDto[]>([])
  const objectsLoading = ref(false)
  const restoreAccountId = ref(0)

  /** 拉远端对象列表；未选账号时直接返回（不发无效请求） */
  async function loadObjects() {
    if (!restoreAccountId.value) return
    objectsLoading.value = true
    try {
      const res = await listRemoteObjects(restoreAccountId.value)
      objects.value = res.data
    } catch (err) {
      toast.error(err, t('common.failed'))
    } finally {
      objectsLoading.value = false
    }
  }

  /**
   * 打开云端恢复弹窗：指向首个启用账号并清空上一轮的远端清单。
   *
   * ⚠️ **切换账号本身不清空清单**——这是原实现的语义（用户须再点「加载远端备份」）。
   * 它对「对着 A 的清单点 B 的还原」没有防护，属已知交互缺口；本次拆分保持原样
   * 并有用例固化，是否改为「切换即清空 / 即重取」见 PR「后续建议」。
   */
  function openRestore(accountId: number) {
    restoreAccountId.value = accountId
    objects.value = []
  }

  return {
    accounts,
    loading,
    fetch,
    selectOptions,
    testingId,
    saving,
    editingId,
    beginEdit,
    save,
    toggle,
    test,
    remove,
    objects,
    objectsLoading,
    restoreAccountId,
    loadObjects,
    openRestore,
  }
}
