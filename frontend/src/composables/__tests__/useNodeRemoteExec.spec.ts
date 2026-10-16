import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第七站的数据层单测（`NodesView` → `useNodeRemoteExec`）。
 *
 * 单节点与批量走**两套成败口径**，这是本站最容易「顺手统一」抹平的地方：
 * - 单节点执行失败要**落进结果区**（退出码 -1）——用户需要看到失败输出，
 *   一闪而过的 toast 留不住原因；
 * - 批量失败走 toast 且**保留**上一轮结果（原实现口径，本轮不改）。
 *
 * 另有两条本地拦截：空命令、未选节点（不发必然 400 的请求）。
 */

const { api, toast } = vi.hoisted(() => ({
  api: { remoteExecute: vi.fn(), batchExecute: vi.fn() },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
}))

vi.mock('@/api/nodes', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/nodes')>()
  return { ...actual, ...api }
})
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))

import { useNodeRemoteExec } from '@/composables/useNodeRemoteExec'

const messages = {
  node: {
    commandRequired: '请输入命令',
    nodeNotSelected: '请至少选择一个节点',
    agentUnreachable: 'Agent 不可达或未配置',
  },
  common: { failed: '操作失败' },
}

function mountExec() {
  const Host = defineComponent({
    setup() {
      return { exec: useNodeRemoteExec() }
    },
    template: '<div/>',
  })
  const w = mount(Host, {
    global: { plugins: [createTestI18n(messages), openVuePlugin, ...services] },
  })
  return (w.vm as unknown as { exec: ReturnType<typeof useNodeRemoteExec> }).exec
}

const row = (over: Record<string, unknown> = {}) => ({
  id: 7,
  name: 'db-1',
  hostname: 'h',
  ip_address: '10.0.0.7',
  status: 'online',
  created_at: '',
  metrics: null,
  online: true,
  ...over,
})

beforeEach(() => {
  vi.clearAllMocks()
  api.remoteExecute.mockResolvedValue({
    data: { output: 'ok', exit_code: 0, duration_ms: 12 },
  })
  api.batchExecute.mockResolvedValue({
    data: { items: [{ node_id: 1, node_name: 'n1', success: true, result: { output: 'ok' } }] },
  })
})

describe('useNodeRemoteExec · 单节点', () => {
  it('工具栏入口不预选节点；行内入口带节点名', () => {
    const exec = mountExec()
    exec.openExecModal()
    expect(exec.execNodeId.value).toBe(0)
    expect(exec.execNodeName.value).toBe('')
    expect(exec.execVisible.value).toBe(true)

    exec.openExecForNode(row() as never)
    expect(exec.execNodeId.value).toBe(7)
    expect(exec.execNodeName.value).toBe('db-1')
  })

  it('【关键】重开弹窗清空上一次的命令与结果（不残留）', async () => {
    const exec = mountExec()
    exec.openExecForNode(row() as never)
    exec.execCommand.value = 'uptime'
    await exec.run()
    expect(exec.execResult.value?.exit_code).toBe(0)

    exec.openExecForNode(row() as never)
    expect(exec.execCommand.value).toBe('')
    expect(exec.execResult.value).toBeNull()
  })

  it('空命令：本地拦下并给字段红字（不发请求）', async () => {
    const exec = mountExec()
    exec.openExecForNode(row() as never)
    exec.execCommand.value = '   '
    await exec.run()

    expect(exec.execErrors.command).toBe('请输入命令')
    expect(api.remoteExecute).not.toHaveBeenCalled()
  })

  it('未选节点（工具栏入口直接执行）：toast 提示（不发请求）', async () => {
    const exec = mountExec()
    exec.openExecModal()
    exec.execCommand.value = 'uptime'
    await exec.run()

    expect(toast.error).toHaveBeenCalled()
    expect(api.remoteExecute).not.toHaveBeenCalled()
  })

  it('命令前后空白被裁掉后送后端', async () => {
    const exec = mountExec()
    exec.openExecForNode(row() as never)
    exec.execCommand.value = '  uptime -a  '
    await exec.run()
    expect(api.remoteExecute).toHaveBeenCalledWith(7, 'uptime -a', 30)
  })

  it('【关键】执行失败也写进结果区（退出码 -1），而不是只 toast——用户要看到失败输出', async () => {
    api.remoteExecute.mockRejectedValue(new Error('agent down'))
    const exec = mountExec()
    exec.openExecForNode(row() as never)
    exec.execCommand.value = 'uptime'
    await exec.run()

    expect(exec.execResult.value).toMatchObject({ exit_code: -1, duration_ms: 0 })
    expect(exec.execResult.value?.output).toBeTruthy()
    // 失败不进 toast（原因已在结果区）
    expect(toast.error).not.toHaveBeenCalled()
    expect(exec.execLoading.value).toBe(false)
  })
})

describe('useNodeRemoteExec · 批量', () => {
  const nodes = [row({ id: 1 }), row({ id: 2, name: 'n2' })] as never[]

  it('【关键】打开即全选当前页（批量场景的默认预期就是这一批一起跑）', () => {
    const exec = mountExec()
    exec.openBatchModal(nodes)
    expect(exec.batchSelectedIds.value).toEqual([1, 2])
    expect(exec.batchCommand.value).toBe('')
  })

  it('重开清空上一批结果（不残留）', async () => {
    const exec = mountExec()
    exec.openBatchModal(nodes)
    exec.batchCommand.value = 'uptime'
    await exec.runBatch()
    expect(exec.batchResult.value).toHaveLength(1)

    exec.openBatchModal(nodes)
    expect(exec.batchResult.value).toEqual([])
  })

  it('空命令 / 未勾选：各自本地拦下且不发请求', async () => {
    const exec = mountExec()
    exec.openBatchModal(nodes)

    exec.batchCommand.value = ''
    await exec.runBatch()
    expect(api.batchExecute).not.toHaveBeenCalled()

    exec.batchCommand.value = 'uptime'
    exec.batchSelectedIds.value = []
    await exec.runBatch()
    expect(toast.error).toHaveBeenCalled()
    expect(api.batchExecute).not.toHaveBeenCalled()
  })

  it('成功：逐节点结果落库，命令裁空白后送后端', async () => {
    const exec = mountExec()
    exec.openBatchModal(nodes)
    exec.batchCommand.value = ' uptime '
    await exec.runBatch()

    expect(api.batchExecute).toHaveBeenCalledWith([1, 2], 'uptime', 30)
    expect(exec.batchResult.value[0].node_name).toBe('n1')
    expect(exec.batchLoading.value).toBe(false)
  })

  it('【口径】传输层失败只 toast，**保留**上一轮结果（原实现行为，已记入后续建议）', async () => {
    const exec = mountExec()
    exec.openBatchModal(nodes)
    exec.batchCommand.value = 'uptime'
    await exec.runBatch()
    const before = exec.batchResult.value

    api.batchExecute.mockRejectedValue(new Error('500'))
    await exec.runBatch()

    expect(toast.error).toHaveBeenCalled()
    expect(exec.batchResult.value).toEqual(before)
  })
})
