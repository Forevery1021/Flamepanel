import { describe, it, expect, vi, beforeEach } from 'vitest'
import { nextTick } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

const transferFiles = vi.fn()
vi.mock('@/api/files', () => ({
  transferFiles: (...a: unknown[]) => transferFiles(...a),
}))

const toast = { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() }
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))

import FileTransferModal from '@/components/files/FileTransferModal.vue'

/**
 * 批量复制 / 移动弹窗契约：
 * 1. 目标目录默认取当前目录，提交 `(mode, sources, dest, overwrite)`；
 * 2. 全部成功 → toast + 关闭 + emit done；
 * 3. 部分失败 → 保持打开并逐条列出原因（冲突给出友好提示），已成功部分仍 emit done 刷新列表；
 * 4. 目标不是绝对路径时禁用提交。
 */

const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

const messages = {
  file: {
    copy: '复制',
    move: '移动',
    copyTo: '复制到',
    moveTo: '移动到',
    targetDir: '目标目录',
    transferItems: '{n} 项',
    transferPartial: '成功 {ok}，失败 {failed}',
    transferConflict: '目标已存在',
    copyDone: '已复制 {n} 项',
    moveDone: '已移动 {n} 项',
    replaceExisting: '替换',
    cancel: '取消',
  },
  common: { failed: '失败' },
}

async function mountModal(mode: 'copy' | 'move' = 'copy') {
  document.body.innerHTML = ''
  const w = mountLoose(FileTransferModal, {
    props: { modelValue: true, mode, dir: '/data', paths: ['/data/a.txt', '/data/b'] },
    attachTo: document.body,
    global: { plugins: [createTestI18n(messages), openVuePlugin, ...services] },
  })
  await flushPromises()
  await nextTick()
  return w
}

function input(): HTMLInputElement {
  return document.body.querySelector(
    '.modal-form input[type="text"], .modal-form input:not([type])',
  ) as HTMLInputElement
}
function submitButton(label: string): HTMLButtonElement {
  const btn = Array.from(document.body.querySelectorAll('button')).find(
    (b) => b.textContent?.trim() === label,
  )
  if (!btn) throw new Error(`button ${label} not found`)
  return btn as HTMLButtonElement
}

beforeEach(() => {
  vi.clearAllMocks()
})

describe('FileTransferModal', () => {
  it('默认目标为当前目录；全部成功后 toast、关闭并通知刷新', async () => {
    transferFiles.mockResolvedValue({
      data: {
        succeeded: 2,
        failed: 0,
        items: [
          { source: '/data/a.txt', ok: true, target: '/data/a (副本).txt' },
          { source: '/data/b', ok: true, target: '/data/b (副本)' },
        ],
      },
    })
    const w = await mountModal('copy')
    expect(input().value).toBe('/data')
    submitButton('复制').click()
    await flushPromises()
    expect(transferFiles).toHaveBeenCalledWith('copy', ['/data/a.txt', '/data/b'], '/data', false)
    expect(toast.success).toHaveBeenCalledWith('已复制 2 项')
    expect(w.emitted('done')).toHaveLength(1)
    expect(w.emitted('update:modelValue')?.at(-1)).toEqual([false])
  })

  it('部分失败：保持打开并列出失败原因，仍通知刷新', async () => {
    transferFiles.mockResolvedValue({
      data: {
        succeeded: 1,
        failed: 1,
        items: [
          { source: '/data/a.txt', ok: true, target: '/srv/a.txt' },
          { source: '/data/b', ok: false, error_kind: 'conflict', error: 'exists' },
        ],
      },
    })
    const w = await mountModal('move')
    const el = input()
    el.value = '/srv'
    el.dispatchEvent(new Event('input'))
    await nextTick()
    submitButton('移动').click()
    await flushPromises()
    expect(transferFiles).toHaveBeenCalledWith('move', ['/data/a.txt', '/data/b'], '/srv', false)
    expect(toast.success).not.toHaveBeenCalled()
    expect(w.emitted('done')).toHaveLength(1)
    expect(w.emitted('update:modelValue')).toBeUndefined()
    expect(document.body.textContent).toContain('成功 1，失败 1')
    expect(document.body.textContent).toContain('目标已存在')
  })

  it('目标不是绝对路径时禁用提交', async () => {
    await mountModal()
    const el = input()
    el.value = 'relative/dir'
    el.dispatchEvent(new Event('input'))
    await nextTick()
    expect(submitButton('复制').disabled).toBe(true)
    expect(transferFiles).not.toHaveBeenCalled()
  })
})
