import { describe, it, expect, vi, beforeEach } from 'vitest'
import { nextTick } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * FilesView 拆出的三个表单弹窗：新建 / 重命名 / 改权限。
 * 契约：打开时预填；空值禁用提交；成功 → toast + 关闭 + emit done；失败保持打开。
 */

const { api, toast } = vi.hoisted(() => ({
  api: { createFile: vi.fn(), createDir: vi.fn(), renameFile: vi.fn(), chmodFile: vi.fn() },
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
}))
vi.mock('@/api/files', () => api)
vi.mock('@/components/ui/FpToast', () => ({ useFpToast: () => toast }))

import FileCreateModal from '@/components/files/FileCreateModal.vue'
import FileRenameModal from '@/components/files/FileRenameModal.vue'
import FileChmodModal from '@/components/files/FileChmodModal.vue'

const messages = {
  file: {
    createDir: '新建目录',
    createFile: '新建文件',
    createIn: '位于 {dir}',
    name: '名称',
    rename: '重命名',
    newName: '新名称',
    chmod: '修改权限',
    mode: '权限',
    cancel: '取消',
  },
  common: { confirm: '确认', success: '成功', failed: '失败' },
}

const mountLoose = mount as unknown as (
  c: unknown,
  o: Record<string, unknown>,
) => ReturnType<typeof mount>

async function open(component: unknown, props: Record<string, unknown>) {
  document.body.innerHTML = ''
  const w = mountLoose(component, {
    props: { modelValue: false, ...props },
    attachTo: document.body,
    global: { plugins: [createTestI18n(messages), openVuePlugin, ...services] },
  })
  await w.setProps({ modelValue: true })
  await flushPromises()
  await nextTick()
  return w
}

const input = () => document.body.querySelector('.modal-form input') as HTMLInputElement
function button(label: string): HTMLButtonElement {
  const b = Array.from(document.body.querySelectorAll('button')).find(
    (x) => x.textContent?.trim() === label,
  )
  if (!b) throw new Error(`button ${label} not found`)
  return b as HTMLButtonElement
}
async function type(value: string) {
  const el = input()
  el.value = value
  el.dispatchEvent(new Event('input'))
  await nextTick()
}

beforeEach(() => {
  vi.clearAllMocks()
  for (const fn of Object.values(api)) fn.mockResolvedValue({ data: {} })
})

describe('FileCreateModal', () => {
  it('目录：空名禁用，提交拼接当前目录并 emit done', async () => {
    const w = await open(FileCreateModal, { kind: 'dir', dir: '/srv' })
    expect(document.body.textContent).toContain('新建目录')
    expect(document.body.textContent).toContain('位于 /srv')
    expect(button('确认').disabled).toBe(true)
    await type('  logs ')
    button('确认').click()
    await flushPromises()
    expect(api.createDir).toHaveBeenCalledWith('/srv/logs')
    expect(api.createFile).not.toHaveBeenCalled()
    expect(w.emitted('done')).toHaveLength(1)
    expect(w.emitted('update:modelValue')?.at(-1)).toEqual([false])
  })

  it('文件：失败报错并保持打开', async () => {
    api.createFile.mockRejectedValue(new Error('exists'))
    const w = await open(FileCreateModal, { kind: 'file', dir: '/' })
    await type('a.txt')
    button('确认').click()
    await flushPromises()
    expect(api.createFile).toHaveBeenCalledWith('/a.txt')
    expect(toast.error).toHaveBeenCalled()
    expect(w.emitted('done')).toBeUndefined()
  })
})

describe('FileRenameModal', () => {
  it('预填原名，提交同目录新路径', async () => {
    const w = await open(FileRenameModal, { file: { path: '/srv/old.txt', name: 'old.txt' } })
    expect(input().value).toBe('old.txt')
    await type('new.txt')
    button('重命名').click()
    await flushPromises()
    expect(api.renameFile).toHaveBeenCalledWith('/srv/old.txt', '/srv/new.txt')
    expect(toast.success).toHaveBeenCalledWith('成功')
    expect(w.emitted('done')).toHaveLength(1)
  })

  it('无目标文件时不提交', async () => {
    await open(FileRenameModal, { file: null })
    await type('x')
    button('重命名').click()
    await flushPromises()
    expect(api.renameFile).not.toHaveBeenCalled()
  })
})

describe('FileChmodModal', () => {
  it('预填当前权限，提交新权限；失败不 emit', async () => {
    const w = await open(FileChmodModal, { file: { path: '/srv/run.sh', permissions: '644' } })
    expect(input().value).toBe('644')
    await type('755')
    button('确认').click()
    await flushPromises()
    expect(api.chmodFile).toHaveBeenCalledWith('/srv/run.sh', '755')
    expect(w.emitted('done')).toHaveLength(1)

    api.chmodFile.mockRejectedValue(new Error('denied'))
    await w.setProps({ modelValue: false })
    await w.setProps({ modelValue: true })
    await flushPromises()
    button('确认').click()
    await flushPromises()
    expect(toast.error).toHaveBeenCalled()
    expect(w.emitted('done')).toHaveLength(1)
  })

  it('权限缺省回落 755，清空后禁用', async () => {
    await open(FileChmodModal, { file: { path: '/x', permissions: '' } })
    expect(input().value).toBe('755')
    await type(' ')
    expect(button('确认').disabled).toBe(true)
  })
})
