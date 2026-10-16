import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import { useApiQuery, useQueryCacheClient } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import {
  listBackups,
  createBackup,
  downloadBackup,
  restoreBackup,
  deleteBackup,
} from '@/api/backups'
import type { BackupEntry } from '@/api/backups'

/**
 * 本地备份数据层（Doc/23 §3.3 T2-4 视图拆分第四站）。
 *
 * 原 BackupView（905 行）把「本地备份 / 备份账号 / 计划任务」三个页签的表格、
 * 弹窗与请求编排揉在同一文件。本 composable 收敛第一族——本地备份文件：
 *
 * - 列表（`useApiQuery`，`queryKeys.backups.list`）；
 * - 新建（`creating` 锁工具栏按钮，成功才刷新）；
 * - 下载（**不刷新**：下载不改服务端状态）；
 * - 恢复 / 删除（带二次确认，成功才刷新）；
 * - 跨族失效 `invalidateAll`：三张表同属「备份域」，工具栏刷新一次刷全部
 *   （原实现如此），故它由本层导出、装配层传给另外两族复用。
 *
 * 行为与原实现逐行对齐（含文案 key、确认分支、失败不刷新的口径），无契约变更。
 */
export function useBackupLocal() {
  const { t } = useI18n()
  const toast = useFpToast()
  const { confirmAction } = useFpConfirm()
  const queryClient = useQueryCacheClient()

  const creating = ref(false)

  const backupsQuery = useApiQuery<BackupEntry[]>(
    () => queryKeys.backups.list(),
    async () => {
      const res = await listBackups()
      return { data: res.data }
    },
  )
  const backups = computed<BackupEntry[]>(() => backupsQuery.data.value ?? [])
  const loading = backupsQuery.loading

  /** 备份域三张表全量失效（工具栏的刷新按钮一次刷三族） */
  function invalidateAll() {
    void queryClient.invalidateQueries({ queryKey: queryKeys.backups.all })
    void queryClient.invalidateQueries({ queryKey: queryKeys.backupAccounts.all })
    void queryClient.invalidateQueries({ queryKey: queryKeys.backupJobs.all })
  }

  function fetch() {
    return backupsQuery.refresh()
  }

  /** 新建备份；成功返回 true（工具栏据此收敛 loading） */
  async function create(): Promise<boolean> {
    creating.value = true
    try {
      await createBackup()
      toast.success(t('backup.createSuccess'))
      invalidateAll()
      return true
    } catch (err) {
      toast.error(err, t('common.failed'))
      return false
    } finally {
      creating.value = false
    }
  }

  /** 下载：浏览器侧触发保存，不改服务端状态，故不刷新 */
  async function download(filename: string) {
    try {
      await downloadBackup(filename)
    } catch (err) {
      toast.error(err, t('common.failed'))
    }
  }

  function restore(filename: string) {
    confirmAction({
      message: t('backup.restoreConfirm', { name: filename }),
      header: t('common.confirm'),
      acceptLabel: t('backup.restore'),
      rejectLabel: t('common.cancel'),
      accept: async () => {
        try {
          await restoreBackup(filename)
          toast.success(t('backup.restoreSuccess'))
        } catch (err) {
          toast.error(err, t('common.failed'))
        }
      },
    })
  }

  function remove(filename: string) {
    confirmAction({
      message: t('backup.deleteConfirm', { name: filename }),
      header: t('common.confirm'),
      acceptLabel: t('backup.delete'),
      rejectLabel: t('common.cancel'),
      accept: async () => {
        try {
          await deleteBackup(filename)
          toast.success(t('common.success'))
          invalidateAll()
        } catch (err) {
          toast.error(err, t('common.failed'))
        }
      },
    })
  }

  return {
    backups,
    loading,
    creating,
    fetch,
    invalidateAll,
    create,
    download,
    restore,
    remove,
  }
}
