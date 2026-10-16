import { describe, it, expect, vi } from 'vitest'
import { nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import NodeFormDialog from '@/components/node/NodeFormDialog.vue'
import NodeExecDialog from '@/components/node/NodeExecDialog.vue'
import NodeBatchExecDialog from '@/components/node/NodeBatchExecDialog.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第七站拆出的三个弹窗契约（原为 NodesView 内联 FpModal）。
 *
 * 本 spec 的重点：
 * - **表单弹窗的字段红字与上抛分流**：注册与编辑原先是两个几乎逐行相同的弹窗，
 *   合并后必须保证「编辑不误发 create」「新建不误发 update」这条分流仍然成立；
 * - **命令弹窗的结果区三态**：未执行不渲染、成败都渲染（失败是 `exit_code: -1`）；
 * - **批量弹窗的勾选模型**：`v-model:selected-ids` 双向可用，且结果逐节点渲染。
 *
 * 弹窗经 Teleport 到 body，断言一律落在 `document.body`（T1-1 已记录的陷阱）。
 */

const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

const messages = {
  node: {
    register: '注册节点',
    editNode: '编辑节点',
    name: '名称',
    hostname: '主机名',
    ip: 'IP 地址',
    status: '状态',
    nameRequired: '请输入节点名称',
    hostnameRequired: '请输入主机名',
    ipRequired: '请输入 IP 地址',
    remoteExec: '远程命令',
    command: '命令',
    commandPlaceholder: '输入要在节点上执行的命令，如: uptime',
    exitCode: '退出码',
    duration: '耗时',
    run: '执行',
    batchExec: '批量命令',
    selectNodesHint: '勾选需要执行命令的节点',
    runBatch: '批量执行',
  },
  dashboard: { online: '在线', offline: '离线' },
  common: { cancel: '取消', confirm: '确定', close: '关闭' },
}

function mountDialog(component: unknown, props: Record<string, unknown> = {}) {
  document.body.innerHTML = ''
  return mountLoose(component, {
    props,
    attachTo: document.body,
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
      directives: { permission: () => {}, tooltip: () => {} },
    },
  })
}

async function settle() {
  await nextTick()
  await nextTick()
  await nextTick()
}

const row = (over: Record<string, unknown> = {}) => ({
  id: 3,
  name: 'db-1',
  hostname: 'db-1.internal',
  ip_address: '10.0.0.1',
  status: 'online',
  created_at: '',
  ...over,
})

/** 批量填三个必填框（input 事件驱动 v-model） */
function fillInputs(values: { name?: string; hostname?: string; ip?: string }) {
  const inputs = Array.from(document.body.querySelectorAll('input')) as HTMLInputElement[]
  const pairs: [string | undefined, HTMLInputElement][] = [
    [values.name, inputs[0]],
    [values.hostname, inputs[1]],
    [values.ip, inputs[2]],
  ]
  for (const [value, input] of pairs) {
    if (value === undefined) continue
    input.value = value
    input.dispatchEvent(new Event('input'))
  }
}

function buttonByText(text: string): HTMLButtonElement {
  const btn = Array.from(document.body.querySelectorAll('button')).find((b) =>
    b.textContent?.includes(text),
  )
  if (!btn) throw new Error(`按钮未找到：${text}`)
  return btn as HTMLButtonElement
}

describe('NodeFormDialog · 打开与初值', () => {
  it('初始不渲染（visible=false）', async () => {
    mountDialog(NodeFormDialog)
    await settle()
    expect(document.body.querySelector('.p-dialog')).toBeNull()
  })

  it('新建：标题「注册节点」，四个字段齐备且默认 online', async () => {
    const w = mountDialog(NodeFormDialog)
    ;(w.vm as unknown as { open: (r?: unknown) => void }).open()
    await settle()

    expect(document.body.textContent).toContain('注册节点')
    for (const label of ['名称', '主机名', 'IP 地址', '状态']) {
      expect(document.body.textContent).toContain(label)
    }
    const inputs = Array.from(document.body.querySelectorAll('input')) as HTMLInputElement[]
    expect(inputs[0].value).toBe('')
  })

  it('【关键】编辑：标题「编辑节点」且四个字段按行数据回填', async () => {
    const w = mountDialog(NodeFormDialog)
    ;(w.vm as unknown as { open: (r: unknown) => void }).open(row())
    await settle()

    expect(document.body.textContent).toContain('编辑节点')
    const inputs = Array.from(document.body.querySelectorAll('input')) as HTMLInputElement[]
    expect(inputs[0].value).toBe('db-1')
    expect(inputs[1].value).toBe('db-1.internal')
    expect(inputs[2].value).toBe('10.0.0.1')
  })

  it('【关键】重开新建会清掉上一次的编辑目标（不残留 id 与字段值）', async () => {
    const w = mountDialog(NodeFormDialog)
    ;(w.vm as unknown as { open: (r: unknown) => void }).open(row({ id: 9 }))
    await settle()
    ;(w.vm as unknown as { open: (r?: unknown) => void }).open()
    await settle()

    const inputs = Array.from(document.body.querySelectorAll('input')) as HTMLInputElement[]
    expect(inputs[0].value, '字段未清空').toBe('')

    // 空表单点确定：三项必填拦住（既不 create 也不 update）
    buttonByText('确定').click()
    await settle()
    let emitted = w.emitted() as Record<string, unknown[][]>
    expect(emitted.update).toBeFalsy()
    expect(emitted.create).toBeFalsy()

    // 填齐后再点：上抛的是 create 而不是 update（编辑目标确已丢弃）
    fillInputs({ name: 'n9', hostname: 'h9', ip: '10.0.0.9' })
    await settle()
    buttonByText('确定').click()
    await settle()
    emitted = w.emitted() as Record<string, unknown[][]>
    expect(emitted.update).toBeFalsy()
    expect(emitted.create?.[0][0]).toMatchObject({ name: 'n9' })
  })
})

describe('NodeFormDialog · 本地校验与上抛', () => {
  it('【关键】空字段：三项红字齐出且不上抛（不发必然 400 的请求）', async () => {
    const w = mountDialog(NodeFormDialog)
    ;(w.vm as unknown as { open: () => void }).open()
    await settle()

    buttonByText('确定').click()
    await settle()

    const text = document.body.textContent ?? ''
    expect(text).toContain('请输入节点名称')
    expect(text).toContain('请输入主机名')
    expect(text).toContain('请输入 IP 地址')
    expect(w.emitted('create')).toBeFalsy()
  })

  it('填齐后新建上抛 create（形状与数据层请求体一致）', async () => {
    const w = mountDialog(NodeFormDialog)
    ;(w.vm as unknown as { open: () => void }).open()
    await settle()

    fillInputs({ name: 'n1', hostname: 'h1', ip: '10.0.0.1' })
    await settle()

    buttonByText('确定').click()
    await settle()

    const created = (w.emitted() as Record<string, unknown[][]>).create
    expect(created).toBeTruthy()
    expect(created![0][0]).toEqual({
      name: 'n1',
      hostname: 'h1',
      ip_address: '10.0.0.1',
      status: 'online',
    })
  })

  it('【关键】编辑上抛 update(id, form)（id 来自打开时的行）', async () => {
    const w = mountDialog(NodeFormDialog)
    ;(w.vm as unknown as { open: (r: unknown) => void }).open(row({ id: 42, status: 'offline' }))
    await settle()

    buttonByText('确定').click()
    await settle()

    const updated = (w.emitted() as Record<string, unknown[][]>).update
    expect(updated![0][0]).toBe(42)
    expect(updated![0][1]).toEqual({
      name: 'db-1',
      hostname: 'db-1.internal',
      ip_address: '10.0.0.1',
      status: 'offline',
    })
  })

  it('close()：隐藏弹窗（父级在请求成功后调用）', async () => {
    const w = mountDialog(NodeFormDialog)
    ;(w.vm as unknown as { open: () => void }).open()
    await settle()
    expect(document.body.querySelector('.p-dialog')).not.toBeNull()

    ;(w.vm as unknown as { close: () => void }).close()
    await settle()
    expect(document.body.querySelector('.p-dialog')).toBeNull()
  })

  it('提交态由 props 回传（确定按钮转圈）', async () => {
    const w = mountDialog(NodeFormDialog, { submitting: true })
    ;(w.vm as unknown as { open: () => void }).open()
    await settle()
    const buttons = w.findAllComponents({ name: 'FpButton' })
    // 最后一个按钮是「确定」（第一个是「取消」）
    expect(buttons[buttons.length - 1].props('loading')).toBe(true)
  })
})

describe('NodeExecDialog · 结果区三态', () => {
  it('未执行：不渲染结果区（只有命令输入框）', async () => {
    mountDialog(NodeExecDialog, { modelValue: true, model: '' })
    await settle()
    expect(document.body.textContent).toContain('远程命令')
    expect(document.body.querySelector('.exec-result')).toBeNull()
  })

  it('【关键】执行成功：渲染退出码与耗时 + 输出正文', async () => {
    mountDialog(NodeExecDialog, {
      modelValue: true,
      nodeName: 'db-1',
      result: { output: 'load average: 0.1', exit_code: 0, duration_ms: 42 },
    })
    await settle()

    const text = document.body.textContent ?? ''
    expect(text).toContain('db-1')
    expect(text).toContain('退出码: 0')
    expect(text).toContain('42ms')
    expect(document.body.querySelector('.exec-output')?.textContent).toContain('load average')
  })

  it('【关键】执行失败也渲染结果区（exit_code -1 + 错误文案）', async () => {
    mountDialog(NodeExecDialog, {
      modelValue: true,
      result: { output: 'Agent 不可达', exit_code: -1, duration_ms: 0 },
    })
    await settle()

    const text = document.body.textContent ?? ''
    expect(text).toContain('退出码: -1')
    expect(document.body.querySelector('.exec-output')?.textContent).toContain('Agent 不可达')
  })

  it('输出为空时渲染 —（不留一段空白 pre）', async () => {
    mountDialog(NodeExecDialog, {
      modelValue: true,
      result: { output: '', exit_code: 0, duration_ms: 1 },
    })
    await settle()
    expect(document.body.querySelector('.exec-output')?.textContent).toBe('—')
  })

  it('【关键】点执行把**输入框里的当前文本**上抛（不存在父级副本）', async () => {
    const w = mountDialog(NodeExecDialog, { modelValue: true, command: 'uptime' })
    await settle()

    buttonByText('执行').click()
    await settle()
    expect((w.emitted() as Record<string, unknown[][]>).run?.[0]).toEqual(['uptime'])
  })

  it('字段错误渲染红字（本地校验或后端拒绝）', async () => {
    mountDialog(NodeExecDialog, { modelValue: true, error: '请输入命令' })
    await settle()
    expect(document.body.textContent).toContain('请输入命令')
  })
})

describe('NodeBatchExecDialog · 勾选与逐节点结果', () => {
  const nodes = [
    { id: 1, name: 'n1', ip_address: '10.0.0.1' },
    { id: 2, name: 'n2', ip_address: '10.0.0.2' },
  ]

  it('渲染候选节点（名称 + IP）与提示文案', async () => {
    mountDialog(NodeBatchExecDialog, { modelValue: true, nodes })
    await settle()

    const text = document.body.textContent ?? ''
    expect(text).toContain('勾选需要执行命令的节点')
    expect(text).toContain('n1 (10.0.0.1)')
    expect(text).toContain('n2 (10.0.0.2)')
  })

  it('【关键】勾选状态由 v-model:selectedIds 驱动（数据层预置全选）', async () => {
    const w = mountDialog(NodeBatchExecDialog, {
      modelValue: true,
      nodes,
      selectedIds: [1, 2],
    })
    await settle()

    const boxes = Array.from(document.body.querySelectorAll('input[type="checkbox"]')) as HTMLInputElement[]
    expect(boxes).toHaveLength(2)
    expect(boxes.every((b) => b.checked)).toBe(true)
    expect(w.props('selectedIds' as never)).toEqual([1, 2])
  })

  it('未跑过：不渲染结果区', async () => {
    mountDialog(NodeBatchExecDialog, { modelValue: true, nodes })
    await settle()
    expect(document.body.querySelector('.batch-result')).toBeNull()
  })

  it('【关键】结果逐节点渲染，输出优先 output、其次 error、最后 —', async () => {
    mountDialog(NodeBatchExecDialog, {
      modelValue: true,
      nodes,
      result: [
        { node_id: 1, node_name: 'n1', success: true, result: { output: 'ok-1' } },
        { node_id: 2, node_name: 'n2', success: false, result: { error: 'boom' } },
        { node_id: 3, node_name: 'n3', success: false, result: {} },
      ],
    })
    await settle()

    const text = document.body.textContent ?? ''
    expect(text).toContain('[n1]')
    expect(text).toContain('OK')
    expect(text).toContain('[n2]')
    expect(text).toContain('ERR')
    expect(text).toContain('ok-1')
    expect(text).toContain('boom')

    const outputs = Array.from(document.body.querySelectorAll('.exec-output')).map(
      (p) => p.textContent,
    )
    expect(outputs).toEqual(['ok-1', 'boom', '—'])
  })

  it('点批量执行上抛 run（不携带参数，数据层持有勾选与命令）', async () => {
    const w = mountDialog(NodeBatchExecDialog, { modelValue: true, nodes, command: 'uptime' })
    await settle()

    buttonByText('批量执行').click()
    await settle()
    expect(w.emitted('run')).toBeTruthy()
  })

  it('loading 态：执行按钮转圈', async () => {
    const w = mountDialog(NodeBatchExecDialog, { modelValue: true, nodes, loading: true })
    await settle()
    const buttons = w.findAllComponents({ name: 'FpButton' })
    expect(buttons[buttons.length - 1].props('loading')).toBe(true)
  })
})

vi.clearAllMocks()
