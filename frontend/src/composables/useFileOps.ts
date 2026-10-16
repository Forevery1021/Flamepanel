import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { deleteFile, downloadFile } from '@/api/files'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import { useFpToast } from '@/components/ui/FpToast'
import { saveBlob } from '@/components/files/fileUtils'
import type { FileInfo } from '@/types'

/**
 * 文件管理的删除 / 下载动作。
 *
 * 删除默认移入回收站（`permanent = false`），逐个执行：部分失败时逐条报错，
 * 只要有成功项就给出汇总提示；结束后回调 `onChanged`（清空选择 + 刷新列表）。
 */
export function useFileOps(onChanged: () => void) {
  const { t } = useI18n()
  const toast = useFpToast()
  const { confirmAction } = useFpConfirm()

  const downloading = ref('')

  async function download(row: FileInfo) {
    downloading.value = row.path
    try {
      const res = await downloadFile(row.path)
      saveBlob(res.data, row.name)
    } catch (e) {
      toast.error(e, t('common.failed'))
    } finally {
      downloading.value = ''
    }
  }

  /** 返回成功删除的条数 */
  async function removeAll(rows: FileInfo[], permanent: boolean): Promise<number> {
    let failed = 0
    for (const row of rows) {
      try {
        await deleteFile(row.path, row.is_dir, permanent)
      } catch (e) {
        failed++
        toast.error(e, t('common.failed'))
      }
    }
    const ok = rows.length - failed
    if (ok > 0) {
      toast.success(permanent ? t('common.success') : t('file.movedToRecycle', { n: ok }))
    }
    onChanged()
    return ok
  }

  function confirmDelete(rows: FileInfo[]) {
    if (!rows.length) return
    confirmAction({
      header: t('common.confirm'),
      message:
        rows.length === 1
          ? t('file.deleteConfirm', { name: rows[0]!.name })
          : t('file.deleteManyConfirm', { n: rows.length }),
      rejectLabel: t('file.cancel'),
      accept: () => void removeAll(rows, false),
    })
  }

  function confirmPurge(row: FileInfo) {
    confirmAction({
      header: t('common.confirm'),
      message: t('file.permanentConfirm', { name: row.name }),
      rejectLabel: t('file.cancel'),
      accept: () => void removeAll([row], true),
    })
  }

  return { downloading, download, removeAll, confirmDelete, confirmPurge }
}
