import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import { createNode, deleteNode, updateNode } from '@/api/nodes'
import { buildNodeUpdateBody, type NodeFormState } from '@/utils/nodeFormat'
import type { ServerNode } from '@/types'

/**
 * 节点增删改数据层（Doc/23 §3.3 T2-4 视图拆分第七站）。
 *
 * 与数据层的通用边界一致（同 `useProjectList` / `useWebsiteList`）：**表单编辑态
 * 与字段级校验归弹窗组件**——字段旁的红字是展示语义，数据层只收「已是合法表单」
 * 的载荷并组装请求体。故本层的三个动作全部是「发请求 → 反馈 → 刷新」：
 *
 * - `create` / `save` 成功返回 `true`，调用方据此关闭弹窗；**失败留在弹窗里**
 *   （用户填的内容不能丢，只 toast 不关窗）；
 * - `save` 的请求体经 `buildNodeUpdateBody` 归一（只送四个可编辑字段，
 *   不回送 Agent 上报的 `metrics_json` / 心跳快照）；
 * - `remove` 走二次确认，确认文案带节点名——列表里同名机器不算罕见，
 *   没有名字的「确定删除？」在那种场景下等于让用户盲赌。
 */
export function useNodeCrud() {
  const { t } = useI18n()
  const toast = useFpToast()
  const { confirmAction } = useFpConfirm()

  /** 提交态（注册与编辑共用——同一时刻只会开一个弹窗） */
  const submitting = ref(false)

  /** 注册提交；成功返回 true（调用方据此关弹窗并清空表单） */
  async function create(form: NodeFormState, onRefresh: () => void): Promise<boolean> {
    submitting.value = true
    try {
      await createNode({
        name: form.name,
        hostname: form.hostname,
        ip_address: form.ip_address,
        status: form.status,
      })
      toast.success(t('common.success'))
      onRefresh()
      return true
    } catch {
      toast.error(t('common.failed'))
      return false
    } finally {
      submitting.value = false
    }
  }

  /** 编辑保存（`id` 由调用方从行数据取出——它是行身份，不是表单字段） */
  async function save(id: number, form: NodeFormState, onRefresh: () => void): Promise<boolean> {
    submitting.value = true
    try {
      await updateNode(id, buildNodeUpdateBody(id, form))
      toast.success(t('common.success'))
      onRefresh()
      return true
    } catch {
      toast.error(t('common.failed'))
      return false
    } finally {
      submitting.value = false
    }
  }

  /** 删除（二次确认后才发请求） */
  function remove(row: ServerNode, onRefresh: () => void) {
    confirmAction({
      message: t('node.deleteConfirm', { name: row.name }),
      header: t('common.confirm'),
      accept: async () => {
        try {
          await deleteNode(row.id)
          toast.success(t('common.success'))
          onRefresh()
        } catch {
          toast.error(t('common.failed'))
        }
      },
    })
  }

  return { submitting, create, save, remove }
}
