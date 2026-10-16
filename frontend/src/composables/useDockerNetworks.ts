import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import { useDockerResource } from '@/composables/useDockerResource'
import {
  createNetwork,
  removeNetwork,
  connectNetwork,
  disconnectNetwork,
  pruneNetworks,
} from '@/api/docker'
import type { DockerNetwork } from '@/types'

/** 创建网络载荷（表单编辑态在弹窗组件内，提交时上抛） */
export interface NetworkCreatePayload {
  name: string
  driver: string
  subnet: string
}

/**
 * Docker 网络数据层（Doc/23 §3.3 T2-4 视图拆分第三站）。
 *
 * 网络页签的操作比其它页签多一条**状态相关分支**：断开连接作用于「已连接容器」，
 * 后端按容器名寻址，且**无已连接容器时前端直接提示、不发请求**（原实现的
 * `openDisconnect` 语义）。这条分支看似可省略，但发起一次必然 404/400 的请求
 * 只会给用户一条后端错误，不如本地把原因讲清楚——故原样保留并有专门用例锁死。
 *
 * 创建成功后由本层清空表单（`resetForm` 由视图在关闭弹窗时调用），
 * 使「表单初值」这件事只有一个落点。
 */
export function useDockerNetworks() {
  const { t } = useI18n()
  const toast = useFpToast()
  const { confirmAction } = useFpConfirm()

  /** Y3：惰性启用——首次切到网络页签时由视图层置 true */
  const enabled = ref(false)
  const res = useDockerResource('networks', enabled)
  const networks = res.data
  const loading = res.loading
  const error = res.error

  const creating = ref(false)

  /** 创建网络；成功返回 true（调用方据此关弹窗） */
  async function create(payload: NetworkCreatePayload): Promise<boolean> {
    creating.value = true
    try {
      await createNetwork(payload.name.trim(), payload.driver, payload.subnet.trim() || undefined)
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

  function removeWithConfirm(row: DockerNetwork) {
    confirmAction({
      message: t('docker.removeConfirm', { name: row.name }),
      header: t('common.confirmAction'),
      accept: async () => {
        try {
          await removeNetwork(row.id)
          toast.success(t('common.success'))
          await res.refresh()
        } catch (e) {
          toast.error(e, t('common.failed'))
        }
      },
    })
  }

  /** 连接容器：`containerId` 为空时不发请求（调用方先做必填校验） */
  async function connect(networkId: string, containerId: string): Promise<boolean> {
    if (!containerId) return false
    try {
      await connectNetwork(networkId, containerId)
      toast.success(t('common.success'))
      await res.refresh()
      return true
    } catch (e) {
      toast.error(e, t('common.failed'))
      return false
    }
  }

  /**
   * 断开连接：对「第一个已连接容器」执行（原实现语义——网络页签的行内动作
   * 不提供容器选择器，后端按容器名寻址）。无已连接容器时本地提示并返回。
   */
  async function disconnect(row: DockerNetwork): Promise<void> {
    const attached = row.containers || []
    if (!attached.length) {
      toast.info(t('docker.noContainers'))
      return
    }
    try {
      await disconnectNetwork(row.id, attached[0].name)
      toast.success(t('common.success'))
      await res.refresh()
    } catch (e) {
      toast.error(e, t('common.failed'))
    }
  }

  function prune() {
    confirmAction({
      message: t('docker.pruneConfirm', { what: t('docker.networks') }),
      header: t('common.confirmAction'),
      accept: async () => {
        try {
          await pruneNetworks()
          toast.success(t('common.success'))
          await res.refresh()
        } catch (e) {
          toast.error(e, t('common.failed'))
        }
      },
    })
  }

  return {
    networks,
    loading,
    error,
    enabled,
    fetch: res.refresh,
    creating,
    create,
    remove: removeWithConfirm,
    connect,
    disconnect,
    prune,
  }
}
