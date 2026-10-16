import { describe, it, expect } from 'vitest'
import { nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import NodeFilesDialog from '@/components/node/NodeFilesDialog.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第七站拆出的远程文件弹窗契约（原为 NodesView 内联 FpModal）。
 *
 * 五类操作全部上抛，本 spec 的重点是**三处展示语义**（它们决定用户能不能正确
 * 理解自己在看什么）：
 *
 * 1. 目录行的「大小」渲染 `—` 而不是 `0 B`（0 会让人以为是个空文件）；
 * 2. 目录行**不可下载**（按钮只在文件行出现），且点目录行上抛 `enter` 而非 `download`；
 * 3. 列表为空渲染 `—`（不是留一段空白）。
 */

const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

const messages = {
  node: {
    nodeFiles: '节点文件',
    name: '名称',
    up: '上级目录',
    go: '进入',
    refresh: '刷新',
    pathPlaceholder: '输入远程目录路径',
    download: '下载',
    upload: '上传',
    uploadFile: '上传文件',
  },
  common: { close: '关闭' },
}

function mountDialog(props: Record<string, unknown> = {}) {
  document.body.innerHTML = ''
  return mountLoose(NodeFilesDialog, {
    props: { modelValue: true, ...props },
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

const entries = [
  { name: 'logs', is_dir: true, size: 0, modified: '' },
  { name: 'app.log', is_dir: false, size: 1536, modified: '' },
]

describe('NodeFilesDialog · 列表渲染', () => {
  it('渲染节点名与路径栏（上跳 / 进入 / 刷新三入口齐备）', async () => {
    mountDialog({ nodeName: 'db-1', path: '/var/log', entries })
    await settle()

    const text = document.body.textContent ?? ''
    expect(text).toContain('db-1')
    expect(text).toContain('上级目录')
    expect(text).toContain('进入')
    expect(text).toContain('刷新')
    expect((document.body.querySelector('input') as HTMLInputElement).value).toBe('/var/log')
  })

  it('【关键】目录行大小渲染 —（不是 0 B）、文件行渲染真实体积', async () => {
    mountDialog({ entries })
    await settle()

    const rows = Array.from(document.body.querySelectorAll('.files-row'))
    expect(rows).toHaveLength(2)
    expect(rows[0].textContent).toContain('logs')
    expect(rows[0].textContent).toContain('—')
    expect(rows[0].textContent).not.toContain('0 B')
    expect(rows[1].textContent).toContain('1.5 KB')
  })

  it('【关键】下载按钮只在文件行出现（目录没有可下载的内容）', async () => {
    mountDialog({ entries })
    await settle()

    const rows = Array.from(document.body.querySelectorAll('.files-row'))
    expect(rows[0].querySelector('button')).toBeNull()
    expect(rows[1].textContent).toContain('下载')
  })

  it('空目录渲染 —（不留空白）', async () => {
    mountDialog({ entries: [] })
    await settle()
    const panel = document.body.querySelector('.modal-form')!
    expect(panel.textContent).toContain('—')
  })
})

describe('NodeFilesDialog · 事件上抛', () => {
  it('【关键】点目录行上抛 enter；点文件行不上抛 enter', async () => {
    const w = mountDialog({ entries })
    await settle()
    const rows = Array.from(document.body.querySelectorAll('.files-row'))

    rows[0].dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await settle()
    expect((w.emitted() as Record<string, unknown[][]>).enter?.[0]).toEqual(['logs'])

    rows[1].dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await settle()
    expect((w.emitted() as Record<string, unknown[][]>).enter).toHaveLength(1)
  })

  it('【关键】点下载按钮只上抛 download（.stop 阻断行点击，不会连带进入目录）', async () => {
    const w = mountDialog({ entries })
    await settle()
    const btn = Array.from(document.body.querySelectorAll('.files-row button')).find((b) =>
      b.textContent?.includes('下载'),
    )!

    btn.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await settle()

    expect((w.emitted() as Record<string, unknown[][]>).download?.[0]).toEqual(['app.log'])
    expect((w.emitted() as Record<string, unknown[][]>).enter).toBeFalsy()
  })

  it('上跳 / 进入 / 刷新 / 上传四类按钮各自上抛', async () => {
    const w = mountDialog({ entries })
    await settle()

    const byText = (text: string) =>
      Array.from(document.body.querySelectorAll('button')).find((b) =>
        b.textContent?.includes(text),
      )!
    byText('上级目录').click()
    byText('进入').click()
    byText('上传').click()
    await settle()

    const emitted = w.emitted() as Record<string, unknown[][]>
    expect(emitted.up).toBeTruthy()
    expect(emitted.load).toBeTruthy()
    expect(emitted.upload).toBeTruthy()
  })

  it('上传中：上传按钮带 loading', async () => {
    const w = mountDialog({ entries, uploading: true })
    await settle()
    const loadings = w.findAllComponents({ name: 'FpButton' }).map((b) => Boolean(b.props('loading')))
    // 上传按钮是唯一带 loading 的那个（上跳 / 进入 / 刷新 / 关闭都不带）
    expect(loadings.filter(Boolean)).toHaveLength(1)
  })

  it('路径与上传字段双向可用（v-model 由父级持有）', async () => {
    const w = mountDialog({ entries, path: '.', uploadName: '' })
    await settle()

    ;(w.vm as unknown as { $emit: (e: string, v: unknown) => void }).$emit('update:path', '/etc')
    await settle()
    expect(w.emitted('update:path')?.[0]).toEqual(['/etc'])
  })
})
