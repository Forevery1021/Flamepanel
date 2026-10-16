import { describe, it, expect, vi, beforeEach } from 'vitest'
import { defineComponent, nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'

/**
 * 文件管理数据层（FilesView 拆分）：目录导航、按路径查询、多选与刷新后的选择修剪。
 */

const { api } = vi.hoisted(() => ({ api: { listFiles: vi.fn() } }))
vi.mock('@/api/files', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/files')>()
  return { ...actual, ...api }
})

import { useFileBrowser } from '@/composables/useFileBrowser'

const file = (path: string, is_dir = false) => ({
  name: path.split('/').pop() || '/',
  path,
  is_dir,
  is_symlink: false,
  size: 1,
  permissions: '644',
  modified_at: '2026-01-01T00:00:00Z',
})

async function settle() {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 5))
  await nextTick()
}

function mountBrowser() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false, retryDelay: 0 } },
  })
  const Host = defineComponent({
    setup: () => ({ b: useFileBrowser() }),
    template: '<div/>',
  })
  const w = mount(Host, { global: { plugins: [[VueQueryPlugin, { queryClient }]] } })
  return { b: (w.vm as unknown as { b: ReturnType<typeof useFileBrowser> }).b, queryClient }
}

beforeEach(() => {
  api.listFiles.mockReset()
  api.listFiles.mockImplementation((path: string) =>
    Promise.resolve({
      data: path === '/' ? [file('/etc', true), file('/a.txt')] : [file(`${path}/x.conf`)],
    }),
  )
})

describe('useFileBrowser', () => {
  it('首屏列根目录；面包屑随导航变化；空路径回到根', async () => {
    const { b } = mountBrowser()
    await settle()
    expect(api.listFiles).toHaveBeenCalledWith('/')
    expect(b.entries.value.map((e) => e.path)).toEqual(['/etc', '/a.txt'])
    expect(b.crumbs.value).toEqual([{ label: '/', path: '/' }])

    b.goTo('/etc')
    await settle()
    expect(api.listFiles).toHaveBeenLastCalledWith('/etc')
    expect(b.crumbs.value.map((c) => c.path)).toEqual(['/', '/etc'])
    b.goTo('')
    expect(b.currentPath.value).toBe('/')
  })

  it('多选：单选 / 全选 / 清空；切换目录清空选择', async () => {
    const { b } = mountBrowser()
    await settle()
    b.toggle('/a.txt', true)
    expect(b.selectedRows.value.map((r) => r.name)).toEqual(['a.txt'])
    expect(b.allSelected.value).toBe(false)
    b.toggleAll(true)
    expect(b.allSelected.value).toBe(true)
    b.toggleAll(false)
    expect(b.selected.value).toEqual([])
    b.toggle('/etc', true)
    b.goTo('/etc')
    await nextTick()
    expect(b.selected.value).toEqual([])
    b.toggle('/etc/x.conf', true)
    b.clearSelection()
    expect(b.selected.value).toEqual([])
  })

  it('invalidate 后列表重拉，已消失的选中项被修剪', async () => {
    const { b } = mountBrowser()
    await settle()
    b.toggle('/a.txt', true)
    b.toggle('/etc', true)
    api.listFiles.mockResolvedValue({ data: [file('/etc', true)] })
    b.invalidate()
    await settle()
    expect(b.entries.value).toHaveLength(1)
    expect(b.selected.value).toEqual(['/etc'])
  })

  it('refresh 主动重拉；失败时暴露 error', async () => {
    const { b } = mountBrowser()
    await settle()
    api.listFiles.mockRejectedValue(new Error('boom'))
    await b.refresh()
    await settle()
    expect(b.error.value).toBeTruthy()
  })
})
