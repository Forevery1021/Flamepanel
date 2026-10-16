import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import MemoList from '@/views/MemoList.vue'
import { createTestI18n, openVuePlugin, services } from '@/components/ui/__tests__/helpers'

/**
 * C-3 缺陷（T1-1 发现）的用户可见影响面回归。
 *
 * 历史行为：FpCheckbox 的 `change` 载荷恒为 `true`（底层带的是 DOM Event，
 * `Boolean(event)` 恒真），而 MemoList 用
 * `@change="(v) => toggle(m, Boolean(v))"` 承载「标记完成 / 取消完成」两条语义
 * ——用户取消勾选时仍提交 `done=true`，「取消完成」**永远失效**。
 *
 * 本 spec 从视图层贯穿「MemoList → FpCheckbox → 底层 input → updateMemo」整条链路，
 * 断言取消勾选真实提交 `done: false`——组件级断言（FpCheckbox.spec）证不到
 * 「消费方拿到的值对不对」这一层。
 */

const updateMemo = vi.fn().mockResolvedValue({ data: {} })

vi.mock('@/api/memos', () => ({
  listMemos: vi.fn().mockResolvedValue({
    data: [
      { id: 1, content: '待办一', kind: 'todo', done: true, created_at: '', updated_at: '' },
      { id: 2, content: '待办二', kind: 'todo', done: false, created_at: '', updated_at: '' },
    ],
  }),
  createMemo: vi.fn(),
  updateMemo: (...args: unknown[]) => updateMemo(...args),
  deleteMemo: vi.fn(),
}))

vi.mock('@/components/ui/FpToast', () => ({
  useFpToast: () => ({ success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() }),
}))
vi.mock('@/components/ui/FpConfirm', () => ({ useFpConfirm: () => ({ confirmAction: vi.fn() }) }))

const messages = {
  memo: {
    placeholder: '记点什么',
    showDone: '显示已完成',
    edit: '编辑',
    deleteConfirm: '确认删除？',
  },
  common: {
    add: '添加',
    delete: '删除',
    noData: '暂无数据',
    cancel: '取消',
    save: '保存',
    failed: '操作失败',
  },
}

async function mountMemoList() {
  const w = mount(MemoList, {
    props: { kind: 'todo' },
    global: {
      plugins: [createTestI18n(messages), openVuePlugin, ...services],
    },
  })
  // 等列表请求 resolve + 渲染刷新
  await new Promise((resolve) => setTimeout(resolve, 30))
  await nextTick()
  return w
}

describe('C-3 修复的视图级回归（MemoList 取消完成）', () => {
  beforeEach(() => updateMemo.mockClear())

  it('【核心回归】取消勾选：提交 done=false（历史缺陷下恒提交 true）', async () => {
    const w = await mountMemoList()
    const items = w.findAll('.memo-item')
    expect(items.length).toBe(2)

    // 第一项 done=true：用户取消勾选
    await items[0].find('input[type="checkbox"]').setValue(false)
    await nextTick()

    expect(updateMemo).toHaveBeenCalledWith(1, { done: false })
  })

  it('勾选未完成项：提交 done=true（正向语义未被修复破坏）', async () => {
    const w = await mountMemoList()
    const items = w.findAll('.memo-item')

    await items[1].find('input[type="checkbox"]').setValue(true)
    await nextTick()

    expect(updateMemo).toHaveBeenCalledWith(2, { done: true })
  })
})
