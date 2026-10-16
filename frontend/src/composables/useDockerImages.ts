import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import { useDockerResource } from '@/composables/useDockerResource'
import { removeImage, pullImage, pruneImages } from '@/api/docker'
import type { DockerImage } from '@/types'

/**
 * Docker 镜像数据层（Doc/23 §3.3 T2-4 视图拆分第三站）。
 *
 * 镜像页签的三类操作：拉取（含输入框清空与「后端回显优先」的成功提示）、
 * 删除（二次确认）、清理（二次确认 → prune → 刷新）。
 *
 * `pull` 的提示文案刻意沿用原语义：后端把拉取流水以字符串回显时直接透出，
 * 否则退回落 `common.success`——拉取是长操作（`longApi`），用户需要看到
 * 后端给的实际输出而不是一句笼统的「成功」。
 */
export function useDockerImages() {
  const { t } = useI18n()
  const toast = useFpToast()
  const { confirmAction } = useFpConfirm()

  /** Y3：惰性启用——首次切到镜像页签时由视图层置 true */
  const enabled = ref(false)
  const res = useDockerResource('images', enabled)
  const images = res.data
  const loading = res.loading
  const error = res.error

  const pullName = ref('')
  const pulling = ref(false)

  /** 拉取镜像；成功返回 true（调用方据此清空输入框） */
  async function pull(): Promise<boolean> {
    const image = pullName.value.trim()
    if (!image) return false
    pulling.value = true
    try {
      const r = await pullImage(image)
      toast.success(typeof r.data === 'string' ? r.data : t('common.success'))
      await res.refresh()
      return true
    } catch (e) {
      toast.error(e, t('common.failed'))
      return false
    } finally {
      pulling.value = false
    }
  }

  async function remove(id: string) {
    try {
      await removeImage(id)
      toast.success(t('common.success'))
      await res.refresh()
    } catch (e) {
      toast.error(e, t('common.failed'))
    }
  }

  function removeWithConfirm(row: DockerImage) {
    confirmAction({
      message: t('docker.removeConfirm', { name: row.id }),
      header: t('common.confirmAction'),
      accept: () => remove(row.id),
    })
  }

  function prune() {
    confirmAction({
      message: t('docker.pruneConfirm', { what: t('docker.images') }),
      header: t('common.confirmAction'),
      accept: async () => {
        try {
          await pruneImages()
          toast.success(t('common.success'))
          await res.refresh()
        } catch (e) {
          toast.error(e, t('common.failed'))
        }
      },
    })
  }

  return {
    images,
    loading,
    error,
    enabled,
    fetch: res.refresh,
    pullName,
    pulling,
    pull,
    remove: removeWithConfirm,
    prune,
  }
}
