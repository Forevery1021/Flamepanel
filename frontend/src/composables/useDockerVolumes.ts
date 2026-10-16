import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import { useDockerResource } from '@/composables/useDockerResource'
import { createVolume, removeVolume, pruneVolumes } from '@/api/docker'
import type { DockerVolume } from '@/types'

/** 创建卷载荷（表单编辑态在弹窗组件内，提交时上抛） */
export interface VolumeCreatePayload {
  name: string
  driver: string
}

/**
 * Docker 卷数据层（Doc/23 §3.3 T2-4 视图拆分第三站）。
 *
 * 卷页签是三张「简单表」里唯一按**名字**而非 ID 寻址的（`removeVolume(name)`）——
 * 这是 Docker API 的语义，不是笔误；拆出后该差异在签名上即可见，不必再读调用点。
 */
export function useDockerVolumes() {
  const { t } = useI18n()
  const toast = useFpToast()
  const { confirmAction } = useFpConfirm()

  /** Y3：惰性启用——首次切到卷页签时由视图层置 true */
  const enabled = ref(false)
  const res = useDockerResource('volumes', enabled)
  const volumes = res.data
  const loading = res.loading
  const error = res.error

  const creating = ref(false)

  /** 创建卷；成功返回 true（调用方据此关弹窗） */
  async function create(payload: VolumeCreatePayload): Promise<boolean> {
    creating.value = true
    try {
      await createVolume(payload.name.trim(), payload.driver)
      toast.success(t('common.success'))
      await res.refresh()
      return true
    } catch (e) {
      toast.error(e, t('common.failed'))
      return false
    } finally {
      creating.value = false
    }
  }

  function removeWithConfirm(row: DockerVolume) {
    confirmAction({
      message: t('docker.removeConfirm', { name: row.name }),
      header: t('common.confirmAction'),
      accept: async () => {
        try {
          await removeVolume(row.name)
          toast.success(t('common.success'))
          await res.refresh()
        } catch (e) {
          toast.error(e, t('common.failed'))
        }
      },
    })
  }

  function prune() {
    confirmAction({
      message: t('docker.pruneConfirm', { what: t('docker.volumes') }),
      header: t('common.confirmAction'),
      accept: async () => {
        try {
          await pruneVolumes()
          toast.success(t('common.success'))
          await res.refresh()
        } catch (e) {
          toast.error(e, t('common.failed'))
        }
      },
    })
  }

  return {
    volumes,
    loading,
    error,
    enabled,
    fetch: res.refresh,
    creating,
    create,
    remove: removeWithConfirm,
    prune,
  }
}
