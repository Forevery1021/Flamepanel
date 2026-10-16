import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { createTestI18n } from '@/components/ui/__tests__/helpers'

/**
 * 文件删除 / 下载动作（FilesView 拆分）：确认文案按条数切换、默认进回收站、
 * 部分失败逐条报错但仍汇总成功数、结束后回调刷新。
 */

const { api, toast, confirmState } = vi.hoisted(() => ({
  api: { deleteFile: vi.fn(), downloadFile: vi.fn() },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
  confirmState: { last: null as null | { message: string; accept?: () => void } },
}))

vi.mock('@/api/files', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/files')>()
  return { ...actual, ...api }
})
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))
vi.mock('@/components/ui/FpConfirm', () => ({
  useFpConfirm: () => ({
    confirmAction: (opts: { message: string; accept?: () => void }) => {
      confirmState.last = opts
    },
  }),
}))
const saveBlob = vi.fn()
vi.mock('@/components/files/fileUtils', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/components/files/fileUtils')>()
  return { ...actual, saveBlob: (...a: unknown[]) => saveBlob(...a) }
})

import { useFileOps } from '@/composables/useFileOps'

const messages = {
  common: { confirm: '确认', success: '成功', failed: '失败' },
  file: {
    cancel: '取消',
    deleteConfirm: '删除 {name}？',
    deleteManyConfirm: '删除 {n} 项？',
    permanentConfirm: '永久删除 {name}？',
    movedToRecycle: '已移入回收站 {n} 项',
  },
}

const row = (name: string, is_dir = false) =>
  ({
    name,
    path: `/d/${name}`,
    is_dir,
    is_symlink: false,
    size: 1,
    permissions: '644',
    modified_at: '',
  }) as never

function mountOps() {
  const onChanged = vi.fn()
  const Host = defineComponent({
    setup: () => ({ ops: useFileOps(onChanged) }),
    template: '<div/>',
  })
  const w = mount(Host, { global: { plugins: [createTestI18n(messages)] } })
  return { ops: (w.vm as unknown as { ops: ReturnType<typeof useFileOps> }).ops, onChanged }
}

const flush = () => new Promise((r) => setTimeout(r, 0))

beforeEach(() => {
  vi.clearAllMocks()
  confirmState.last = null
  api.deleteFile.mockResolvedValue({ data: {} })
})

describe('useFileOps', () => {
  it('单条 / 多条确认文案；确认后进回收站并回调', async () => {
    const { ops, onChanged } = mountOps()
    ops.confirmDelete([])
    expect(confirmState.last).toBeNull()

    ops.confirmDelete([row('a.txt')])
    expect(confirmState.last!.message).toBe('删除 a.txt？')
    ops.confirmDelete([row('a.txt'), row('b', true)])
    expect(confirmState.last!.message).toBe('删除 2 项？')
    confirmState.last!.accept!()
    await flush()
    expect(api.deleteFile).toHaveBeenCalledWith('/d/a.txt', false, false)
    expect(api.deleteFile).toHaveBeenCalledWith('/d/b', true, false)
    expect(toast.success).toHaveBeenCalledWith('已移入回收站 2 项')
    expect(onChanged).toHaveBeenCalledTimes(1)
  })

  it('永久删除走 permanent=true', async () => {
    const { ops } = mountOps()
    ops.confirmPurge(row('c.log'))
    expect(confirmState.last!.message).toBe('永久删除 c.log？')
    confirmState.last!.accept!()
    await flush()
    expect(api.deleteFile).toHaveBeenCalledWith('/d/c.log', false, true)
    expect(toast.success).toHaveBeenCalledWith('成功')
  })

  it('部分失败：逐条报错，成功数仍汇总；全部失败不报成功', async () => {
    const { ops, onChanged } = mountOps()
    api.deleteFile.mockRejectedValueOnce(new Error('x')).mockResolvedValueOnce({ data: {} })
    expect(await ops.removeAll([row('a'), row('b')], false)).toBe(1)
    expect(toast.error).toHaveBeenCalledTimes(1)
    expect(toast.success).toHaveBeenCalledWith('已移入回收站 1 项')

    vi.clearAllMocks()
    api.deleteFile.mockRejectedValue(new Error('x'))
    expect(await ops.removeAll([row('a')], false)).toBe(0)
    expect(toast.success).not.toHaveBeenCalled()
    expect(onChanged).toHaveBeenCalled()
  })

  it('下载：保存 blob，进行中标记路径，失败报错并复位', async () => {
    const { ops } = mountOps()
    api.downloadFile.mockResolvedValue({ data: new Blob(['x']) })
    const p = ops.download(row('a.txt'))
    expect(ops.downloading.value).toBe('/d/a.txt')
    await p
    expect(saveBlob).toHaveBeenCalledWith(expect.any(Blob), 'a.txt')
    expect(ops.downloading.value).toBe('')

    api.downloadFile.mockRejectedValue(new Error('no'))
    await ops.download(row('b.txt'))
    expect(toast.error).toHaveBeenCalled()
    expect(ops.downloading.value).toBe('')
  })
})
