import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { defineComponent } from 'vue'
import { mount } from '@vue/test-utils'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * T2-4 第七站的数据层单测（`NodesView` → `useNodeFiles`）。
 *
 * 一个弹窗内的五件事（浏览 / 进入 / 上跳 / 下载 / 上传）全部围绕 `path` 展开，
 * 本 spec 锁住四条边界：
 * ① 打开即拉根目录（不带着上个节点的路径列目录）；
 * ② 失败即清空列表（与列表页的「失败保留旧数据」刻意不同）；
 * ③ 上跳不越过根；
 * ④ 下载/上传的载荷形态（base64 与文件名）。
 */

const { api, toast } = vi.hoisted(() => ({
  api: {
    remoteListFiles: vi.fn(),
    remoteDownloadFile: vi.fn(),
    remoteUploadFile: vi.fn(),
  },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
}))

vi.mock('@/api/nodes', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/nodes')>()
  return { ...actual, ...api }
})
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))

import { useNodeFiles } from '@/composables/useNodeFiles'

const messages = {
  node: {
    agentUnreachable: 'Agent 不可达或未配置',
    fileContent: '文件内容',
  },
  common: { success: '操作成功', failed: '操作失败' },
}

function mountFiles() {
  const Host = defineComponent({
    setup() {
      return { files: useNodeFiles() }
    },
    template: '<div/>',
  })
  const w = mount(Host, {
    global: { plugins: [createTestI18n(messages), openVuePlugin, ...services] },
  })
  return (w.vm as unknown as { files: ReturnType<typeof useNodeFiles> }).files
}

const entry = (over: Record<string, unknown> = {}) => ({
  name: 'app.log',
  is_dir: false,
  size: 1024,
  modified: '2026-01-01',
  ...over,
})

const row = { id: 7, name: 'db-1' } as never

/** `openForNode` 里的首次 load 是 fire-and-forget，断言前先冲掉微任务队列 */
async function flush() {
  await Promise.resolve()
  await Promise.resolve()
  await new Promise((r) => setTimeout(r, 0))
}

beforeEach(() => {
  vi.clearAllMocks()
  api.remoteListFiles.mockResolvedValue({ data: [entry()] })
  api.remoteDownloadFile.mockResolvedValue({ data: { content_base64: 'aGVsbG8=' } })
  api.remoteUploadFile.mockResolvedValue({ data: { node_id: 7, path: 'x', size: 5 } })
})

describe('useNodeFiles · 打开与浏览', () => {
  it('【关键】打开即绑定节点、路径归根、清空上传区，并立刻列根目录', async () => {
    const files = mountFiles()
    files.path.value = '/somewhere/else'
    files.uploadName.value = 'old.txt'
    files.uploadContent.value = 'old'

    files.openForNode(row)
    await flush()
    expect(files.visible.value).toBe(true)
    expect(files.nodeName.value).toBe('db-1')
    expect(files.path.value).toBe('.')
    expect(files.uploadName.value).toBe('')
    expect(files.uploadContent.value).toBe('')
    expect(api.remoteListFiles).toHaveBeenCalledWith(7, '.')
  })

  it('进入子目录：路径拼接后重拉', async () => {
    const files = mountFiles()
    files.openForNode(row)
    await flush()
    api.remoteListFiles.mockClear()

    files.enter('logs')
    expect(files.path.value).toBe('logs')
    expect(api.remoteListFiles).toHaveBeenCalledWith(7, 'logs')
  })

  it('上跳：回到根的上一级仍是 `.`，且重拉', async () => {
    const files = mountFiles()
    files.openForNode(row)
    await flush()
    files.enter('logs')
    api.remoteListFiles.mockClear()

    files.goUp()
    expect(files.path.value).toBe('.')
    expect(api.remoteListFiles).toHaveBeenCalledWith(7, '.')
  })

  it('未绑定节点时不发请求（nodeId 为 0）', async () => {
    const files = mountFiles()
    await files.load()
    expect(api.remoteListFiles).not.toHaveBeenCalled()
  })

  it('【关键】列目录失败即清空列表（留着上个目录的文件名会让人误以为已切过去）', async () => {
    const files = mountFiles()
    files.openForNode(row)
    await flush()
    expect(files.entries.value).toHaveLength(1)

    api.remoteListFiles.mockRejectedValue(new Error('agent down'))
    await files.load()

    expect(files.entries.value).toEqual([])
    expect(toast.error).toHaveBeenCalled()
  })
})

describe('useNodeFiles · 下载', () => {
  it('根目录下拼文件名、解码 base64、触发 a[download] 并释放 URL', async () => {
    const files = mountFiles()
    files.openForNode(row)
    await flush()

    const createSpy = vi.fn(() => 'blob:mock')
    const revokeSpy = vi.fn()
    Object.defineProperty(URL, 'createObjectURL', { value: createSpy, configurable: true })
    Object.defineProperty(URL, 'revokeObjectURL', { value: revokeSpy, configurable: true })
    const clickSpy = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {})

    await files.download('app.log')

    expect(api.remoteDownloadFile).toHaveBeenCalledWith(7, 'app.log')
    expect(createSpy).toHaveBeenCalled()
    expect(clickSpy).toHaveBeenCalled()
    // 用完立即释放，否则每下载一次泄一个 blob URL
    expect(revokeSpy).toHaveBeenCalledWith('blob:mock')

    clickSpy.mockRestore()
  })

  it('子目录下拼完整路径', async () => {
    const files = mountFiles()
    files.openForNode(row)
    await flush()
    files.enter('logs')
    const clickSpy = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {})

    await files.download('app.log')
    expect(api.remoteDownloadFile).toHaveBeenCalledWith(7, 'logs/app.log')

    clickSpy.mockRestore()
  })

  it('下载失败：toast 错误，不抛错', async () => {
    api.remoteDownloadFile.mockRejectedValue(new Error('404'))
    const files = mountFiles()
    files.openForNode(row)
    await flush()

    await files.download('missing.log')
    expect(toast.error).toHaveBeenCalled()
  })
})

describe('useNodeFiles · 上传', () => {
  it('文件名或内容为空：本地拦下且不发请求', async () => {
    const files = mountFiles()
    files.openForNode(row)
    await flush()
    api.remoteUploadFile.mockClear()

    await files.upload()
    expect(toast.error).toHaveBeenCalled()
    expect(api.remoteUploadFile).not.toHaveBeenCalled()

    files.uploadName.value = 'x.txt'
    await files.upload()
    expect(api.remoteUploadFile).not.toHaveBeenCalled()
  })

  it('【关键】内容按 UTF-8 编成 base64 送后端（中文不丢字节）', async () => {
    const files = mountFiles()
    files.openForNode(row)
    await flush()
    files.enter('logs')
    files.uploadName.value = ' 中文.txt '
    files.uploadContent.value = '内容 🚀'

    await files.upload()

    const [id, path, b64] = api.remoteUploadFile.mock.calls[0] as [number, string, string]
    expect(id).toBe(7)
    expect(path).toBe('logs/中文.txt')
    expect(new TextDecoder().decode(Uint8Array.from(atob(b64), (c) => c.charCodeAt(0)))).toBe(
      '内容 🚀',
    )
  })

  it('成功：清空上传区并重拉目录（新文件立刻可见）', async () => {
    const files = mountFiles()
    files.openForNode(row)
    await flush()
    files.uploadName.value = 'x.txt'
    files.uploadContent.value = 'hi'
    api.remoteListFiles.mockClear()

    await files.upload()

    expect(files.uploadName.value).toBe('')
    expect(files.uploadContent.value).toBe('')
    expect(files.uploading.value).toBe(false)
    expect(api.remoteListFiles).toHaveBeenCalled()
  })

  it('上传失败：toast 错误、保留用户填的内容（不能丢）', async () => {
    api.remoteUploadFile.mockRejectedValue(new Error('500'))
    const files = mountFiles()
    files.openForNode(row)
    await flush()
    files.uploadName.value = 'x.txt'
    files.uploadContent.value = 'hi'

    await files.upload()

    expect(toast.error).toHaveBeenCalled()
    expect(files.uploadName.value).toBe('x.txt')
    expect(files.uploadContent.value).toBe('hi')
    expect(files.uploading.value).toBe(false)
  })
})

afterEach(() => {
  vi.restoreAllMocks()
})
