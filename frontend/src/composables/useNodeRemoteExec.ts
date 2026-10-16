import { reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import { batchExecute, remoteExecute } from '@/api/nodes'
import { getErrorMessage } from '@/utils/error'
import type { EnrichedNode } from '@/utils/nodeFormat'
import type { BatchExecItem, RemoteExecResult, ServerNode } from '@/types'

/**
 * 节点远程命令数据层（Doc/23 §3.3 T2-4 视图拆分第七站，Stage5 远程命令/批量）。
 *
 * 两个弹窗共用「命令 + 目标 + 结果区」的骨架，但**执行形态与输入来源不同**：
 *
 * | | 远程命令 | 批量命令 |
 * |---|---|---|
 * | 目标 | 单节点（工具栏进入时**未选**，行内进入时带节点） | 多节点勾选 |
 * | 首次默认 | 无 | **全选当前页**（原实现口径：批量场景本就是「这批一起跑」） |
 * | 失败形态 | **写进结果区**（`exit_code: -1` + 错误文案） | toast 错误，结果区保留上一轮 |
 *
 * ## 两处刻意不统一的语义（拆分时逐条保留）
 *
 * 1. **单节点执行失败不进 toast，而是落到结果区**：远程执行失败（Agent 不可达、
 *    命令超时）与「命令跑完但退出码非 0」在用户眼里是同一件事——都需要看到
 *    所谓「输出」。把它做成一闪而过的 toast，用户回头就找不到失败原因了。
 * 2. **批量失败走 toast、且不清空上一轮结果**（原实现口径，本轮原样保留）：
 *    一次打多台，逐台失败原因是后端在响应体的 `items` 里返回的（HTTP 本身
 *    是 200）；真正的传输层失败才 toast。⚠️ 保留旧结果确实容易让人把上一批的
 *    输出误读成本次结果（与单节点链路的「失败必覆盖结果区」形成对比），
 *    属存量不一致，已记入 PR「后续建议」，不在本轮「零契约变更」范围内。
 *
 * 目标合法性在本地先拦一道（未选节点、空命令），避免发一次必然 400 的请求。
 */
export function useNodeRemoteExec() {
  const { t } = useI18n()
  const toast = useFpToast()

  /* ── 远程命令（单节点） ── */

  const execVisible = ref(false)
  const execNodeId = ref(0)
  const execNodeName = ref('')
  const execCommand = ref('')
  const execErrors = reactive({ command: '' })
  const execLoading = ref(false)
  const execResult = ref<RemoteExecResult | null>(null)

  /** 工具栏入口：不预选节点（用户自己在弹窗里选目标） */
  function openExecModal() {
    execNodeId.value = 0
    execNodeName.value = ''
    execCommand.value = ''
    execErrors.command = ''
    execResult.value = null
    execVisible.value = true
  }

  /** 行内入口：带上该节点 */
  function openExecForNode(row: ServerNode) {
    execNodeId.value = row.id
    execNodeName.value = row.name
    execCommand.value = ''
    execErrors.command = ''
    execResult.value = null
    execVisible.value = true
  }

  async function run() {
    if (!execCommand.value.trim()) {
      execErrors.command = t('node.commandRequired')
      return
    }
    if (execNodeId.value <= 0) {
      toast.error(t('node.nodeNotSelected'))
      return
    }
    execLoading.value = true
    try {
      const res = await remoteExecute(execNodeId.value, execCommand.value.trim(), 30)
      execResult.value = res.data
    } catch (e: unknown) {
      // 失败也进结果区：退出码 -1 是「没跑成」的显式标记（见文件头说明）
      execResult.value = {
        output: getErrorMessage(e, t('node.agentUnreachable')),
        exit_code: -1,
        duration_ms: 0,
      }
    } finally {
      execLoading.value = false
    }
  }

  /* ── 批量命令 ── */

  const batchVisible = ref(false)
  const batchCommand = ref('')
  const batchSelectedIds = ref<number[]>([])
  const batchLoading = ref(false)
  const batchResult = ref<BatchExecItem[]>([])

  /** 打开即**全选当前页**（原实现口径）；本轮结果清空，不残留上一批 */
  function openBatchModal(nodes: EnrichedNode[]) {
    batchCommand.value = ''
    batchSelectedIds.value = nodes.map((n) => n.id)
    batchResult.value = []
    batchVisible.value = true
  }

  async function runBatch() {
    if (!batchCommand.value.trim()) {
      toast.error(t('node.commandRequired'))
      return
    }
    if (!batchSelectedIds.value.length) {
      toast.error(t('node.nodeNotSelected'))
      return
    }
    batchLoading.value = true
    try {
      const res = await batchExecute(batchSelectedIds.value, batchCommand.value.trim(), 30)
      batchResult.value = res.data.items
    } catch (e: unknown) {
      // 传输层失败只 toast，**保留上一轮结果**（原实现口径，本轮不改变行为；
      // 它确实容易让人把旧结果误读成本次结果，已记入 PR「后续建议」）
      toast.error(getErrorMessage(e, t('common.failed')))
    } finally {
      batchLoading.value = false
    }
  }

  return {
    execVisible,
    execNodeId,
    execNodeName,
    execCommand,
    execErrors,
    execLoading,
    execResult,
    openExecModal,
    openExecForNode,
    run,
    batchVisible,
    batchCommand,
    batchSelectedIds,
    batchLoading,
    batchResult,
    openBatchModal,
    runBatch,
  }
}
