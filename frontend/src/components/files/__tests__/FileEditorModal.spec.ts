import { describe, it, expect, vi, beforeEach } from 'vitest'
import { nextTick } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

const readFile = vi.fn()
const writeFile = vi.fn()
vi.mock('@/api/files', () => ({
  readFile: (...a: unknown[]) => readFile(...a),
  writeFile: (...a: unknown[]) => writeFile(...a),
}))

const confirmAction = vi.fn()
vi.mock('@/components/ui/FpConfirm', () => ({ useFpConfirm: () => ({ confirmAction }) }))

import FileEditorModal from '@/components/files/FileEditorModal.vue'

/**
 * 编辑器保存契约（M-C 文件管理）：
 * 1. 保存携带打开时的 `modified_at` 作为 `expected_modified`，成功后以返回值作为下一次基线；
 * 2. 后端 409（文件已被他人修改）→ 弹确认，确认后**不带** `expected_modified` 强制覆盖；
 * 3. Ctrl+S 触发保存；未修改时不发请求。
 */

const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

const messages = {
  file: {
    content: '内容',
    saveShortcut: 'Ctrl+S 保存',
    close: '关闭',
    save: '保存',
    unsaved: '未保存',
    editConflict: '冲突',
    overwrite: '覆盖',
    cancel: '取消',
    discardConfirm: '放弃？',
  },
  common: { success: '成功', failed: '失败', confirm: '确认' },
}

const file = {
  name: 'app.ini',
  path: '/conf/app.ini',
  size: 5,
  is_dir: false,
  permissions: '644',
  modified_at: 'T1',
  mime_type: null,
}

async function mountEditor() {
  document.body.innerHTML = ''
  const w = mountLoose(FileEditorModal, {
    props: { modelValue: true, file },
    attachTo: document.body,
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
      directives: { permission: () => {} },
    },
  })
  await flushPromises()
  await nextTick()
  return w
}

function textarea(): HTMLTextAreaElement {
  return document.body.querySelector('textarea') as HTMLTextAreaElement
}

async function type(value: string) {
  const ta = textarea()
  ta.value = value
  ta.dispatchEvent(new Event('input'))
  await nextTick()
}

async function pressCtrlS() {
  textarea().dispatchEvent(new KeyboardEvent('keydown', { key: 's', ctrlKey: true, bubbles: true }))
  await flushPromises()
}

describe('FileEditorModal · 保存与冲突', () => {
  beforeEach(() => {
    readFile.mockReset().mockResolvedValue({ data: 'hello' })
    writeFile.mockReset()
    confirmAction.mockReset()
  })

  it('打开即读取内容；未修改时 Ctrl+S 不发请求', async () => {
    await mountEditor()
    expect(readFile).toHaveBeenCalledWith('/conf/app.ini')
    expect(textarea().value).toBe('hello')
    await pressCtrlS()
    expect(writeFile).not.toHaveBeenCalled()
  })

  it('保存携带 expected_modified，成功后以新时间为基线', async () => {
    writeFile
      .mockResolvedValueOnce({ data: { modified_at: 'T2', size: 6 } })
      .mockResolvedValueOnce({ data: { modified_at: 'T3', size: 7 } })
    const w = await mountEditor()
    await type('hello!')
    await pressCtrlS()
    expect(writeFile).toHaveBeenLastCalledWith('/conf/app.ini', 'hello!', 'T1')
    expect(w.emitted('saved')).toHaveLength(1)

    await type('hello!!')
    await pressCtrlS()
    expect(writeFile).toHaveBeenLastCalledWith('/conf/app.ini', 'hello!!', 'T2')
  })

  it('【关键】409 → 确认后不带 expected_modified 强制覆盖', async () => {
    writeFile
      .mockRejectedValueOnce(Object.assign(new Error('conflict'), { status: 409 }))
      .mockResolvedValueOnce({ data: { modified_at: 'T9', size: 6 } })
    const w = await mountEditor()
    await type('mine')
    await pressCtrlS()
    expect(confirmAction).toHaveBeenCalledTimes(1)
    expect(w.emitted('saved')).toBeUndefined()

    const opts = confirmAction.mock.calls[0]![0] as { accept: () => void }
    opts.accept()
    await flushPromises()
    expect(writeFile).toHaveBeenLastCalledWith('/conf/app.ini', 'mine', undefined)
    expect(w.emitted('saved')).toHaveLength(1)
  })
})
