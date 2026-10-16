import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import { useApiQuery } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import {
  listBackupJobs,
  createBackupJob,
  updateBackupJob,
  deleteBackupJob,
  toggleBackupJob,
  runBackupJob,
  restoreFromCloud,
} from '@/api/backupJobs'
import type { BackupJobDto } from '@/api/generated'

/** 任务表单载荷（编辑态由弹窗组件持有，提交时按此形状上抛） */
export interface BackupJobFormPayload {
  name: string
  scope: string
  scope_key: string
  schedule: string
  target_account_id: number
  retention: number
  enabled: boolean
}

/**
 * 计划备份任务数据层（Doc/23 §3.3 T2-4 视图拆分第四站）。
 *
 * 保留了原实现里两条容易在拆分中被「顺手修正」的语义：
 *
 * 1. **`scope !== 'app'` 时 `scope_key` 一律上报 `null`**（不是空串）：
 *    协议里 `null` 表示「不适用」，空串会被后端当成一个名为空的应用 key。
 * 2. **`run`（立即执行）取响应体的 `last_status` 判成败**：任务受理成功不代表
 *    备份成功——接口 200 而 `last_status = 'failed'` 是常见形态（sftp 连不上等），
 *    此时必须给错误提示，否则用户以为备好了。
 *
 * 另外云端恢复的「发起」也收在本层：它是任务族的动作（走 `restoreFromCloud`），
 * 只是入口在账号页签的工具栏。
 */
export function useBackupJobs() {
  const { t } = useI18n()
  const toast = useFpToast()
  const { confirmAction } = useFpConfirm()

  const jobsQuery = useApiQuery<BackupJobDto[]>(
    () => queryKeys.backupJobs.list(),
    async () => {
      const res = await listBackupJobs()
      return { data: res.data }
    },
  )
  const jobs = computed<BackupJobDto[]>(() => jobsQuery.data.value ?? [])
  const loading = jobsQuery.loading
  function fetch() {
    return jobsQuery.refresh()
  }

  const saving = ref(false)
  const editingId = ref(0)
  const runningId = ref<number | null>(null)
  const restoringKey = ref<string | null>(null)

  /** 进入编辑态：记录目标 id（`0` 为新建）；语义同账号层 */
  function beginEdit(row?: BackupJobDto) {
    editingId.value = row?.id ?? 0
  }

  /** 保存（新建或编辑）；成功返回 true（调用方据此关弹窗） */
  async function save(payload: BackupJobFormPayload): Promise<boolean> {
    saving.value = true
    const body = {
      name: payload.name,
      scope: payload.scope,
      // 语义 1：非 app 范围显式 null（空串会被当成「名为空的应用」）
      scope_key: payload.scope === 'app' ? payload.scope_key : null,
      schedule: payload.schedule,
      target_account_id: payload.target_account_id,
      retention: payload.retention,
      enabled: payload.enabled,
    }
    try {
      if (editingId.value) {
        await updateBackupJob(editingId.value, body)
      } else {
        await createBackupJob(body)
      }
      toast.success(t('common.success'))
      void jobsQuery.refresh()
      return true
    } catch (err) {
      toast.error(err, t('common.failed'))
      return false
    } finally {
      saving.value = false
    }
  }

  /** 启用 / 停用（二次确认；无论成败都刷新） */
  function toggle(row: BackupJobDto, enabled: boolean) {
    confirmAction({
      message: enabled
        ? t('backup.jobEnableConfirm', { name: row.name })
        : t('backup.jobDisableConfirm', { name: row.name }),
      header: t('common.confirm'),
      accept: async () => {
        try {
          await toggleBackupJob(row.id, enabled)
          toast.success(t('common.success'))
          void jobsQuery.refresh()
        } catch {
          toast.error(t('common.failed'))
          void jobsQuery.refresh()
        }
      },
    })
  }

  /** 立即执行（二次确认；成败取自响应体的 `last_status`——见文件头语义 2） */
  function run(row: BackupJobDto) {
    confirmAction({
      message: t('backup.jobRunConfirm', { name: row.name }),
      header: t('common.confirm'),
      accept: async () => {
        runningId.value = row.id
        try {
          const res = await runBackupJob(row.id)
          if (res.data.last_status === 'success') {
            toast.success(t('backup.jobRunSuccess'))
          } else {
            toast.error(t('backup.jobRunFailed'))
          }
          void jobsQuery.refresh()
        } catch (err) {
          toast.error(err, t('common.failed'))
        } finally {
          runningId.value = null
        }
      },
    })
  }

  function remove(row: BackupJobDto) {
    confirmAction({
      message: t('backup.jobDeleteConfirm', { name: row.name }),
      header: t('common.confirm'),
      acceptLabel: t('common.delete'),
      rejectLabel: t('common.cancel'),
      accept: async () => {
        try {
          await deleteBackupJob(row.id)
          toast.success(t('common.success'))
          void jobsQuery.refresh()
        } catch (err) {
          toast.error(err, t('common.failed'))
        }
      },
    })
  }

  /**
   * 云端恢复（由账号页签的「云端恢复」弹窗发起）——先二次确认，再发起。
   *
   * 两处细节不可省：
   * - 确认文案**按 key 分流**（面板库 / 应用数据）：两种还原的后果完全不同
   *   （前者覆盖面板库、后者整个替换应用数据目录），一句笼统的「确定恢复吗」不够用；
   * - 成功时透出**后端回显的 `message`**（含「是否需重启」的下一步提示），
   *   那句提示无法在本地拼出来——只有后端知道本次还原的是面板库还是应用数据。
   */
  function confirmRestoreCloud(accountId: number, key: string, onDone?: () => void) {
    const isApp = key.startsWith('app/')
    confirmAction({
      message: isApp
        ? t('backup.cloudRestoreAppConfirm', { key })
        : t('backup.cloudRestorePanelConfirm', { key }),
      header: t('common.confirm'),
      acceptLabel: t('backup.restore'),
      rejectLabel: t('common.cancel'),
      accept: async () => {
        restoringKey.value = key
        try {
          const res = await restoreFromCloud(accountId, key)
          toast.success(res.data.message)
          void jobsQuery.refresh()
          onDone?.()
        } catch (err) {
          toast.error(err, t('common.failed'))
        } finally {
          restoringKey.value = null
        }
      },
    })
  }

  return {
    jobs,
    loading,
    fetch,
    saving,
    editingId,
    beginEdit,
    runningId,
    restoringKey,
    save,
    toggle,
    run,
    remove,
    confirmRestoreCloud,
  }
}
