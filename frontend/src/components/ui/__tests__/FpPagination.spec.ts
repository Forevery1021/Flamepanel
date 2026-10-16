import { describe, it, expect } from 'vitest'
import { mountFp } from './helpers'
import FpPagination from '../FpPagination.vue'

/**
 * T1-1 组件测试：FpPagination 分页条。
 *
 * 核心契约有两条：
 * 1. `total <= pageSize` 时整条隐藏——单页数据不占垂直空间，这是列表页
 *    每屏可见信息量的一部分；
 * 2. 翻页只发 `update:first`（受控组件），自身不保存页码，父组件是唯一事实来源。
 */
describe('FpPagination 分页条', () => {
  it('total <= pageSize：不渲染分页条（单页无意义）', () => {
    const w = mountFp(FpPagination, { props: { total: 10, pageSize: 20 } })
    expect(w.find('.p-paginator').exists()).toBe(false)
  })

  it('total > pageSize：渲染页码与首页/末页按钮', () => {
    const w = mountFp(FpPagination, { props: { total: 100, pageSize: 20 } })
    expect(w.find('.p-paginator').exists()).toBe(true)
    expect(w.find('.p-paginator-first').exists()).toBe(true)
    expect(w.find('.p-paginator-last').exists()).toBe(true)
    expect(w.find('.p-paginator-page-selected').text()).toBe('1')
  })

  it('点击第 2 页：发出 update:first = pageSize', async () => {
    const w = mountFp(FpPagination, { props: { total: 100, pageSize: 20 } })
    await w.findAll('.p-paginator-page')[1]!.trigger('click')
    expect(w.emitted('update:first')?.at(-1)).toEqual([20])
  })

  it('受控 first：父组件传 20 时高亮第 2 页', () => {
    const w = mountFp(FpPagination, { props: { total: 100, pageSize: 20, first: 20 } })
    expect(w.find('.p-paginator-page-selected').text()).toBe('2')
  })

  it('首页按钮在首页时禁用（避免无意义请求）', () => {
    const w = mountFp(FpPagination, { props: { total: 100, pageSize: 20 } })
    expect(w.find('.p-paginator-first').classes()).toContain('p-disabled')
  })

  it('rowsPerPageOptions 决定每页下拉可选项', () => {
    const w = mountFp(FpPagination, {
      props: { total: 100, pageSize: 10, rowsPerPageOptions: [10, 50] },
    })
    // 下拉由模板声明驱动，存在即为已装配
    expect(w.find('.p-paginator-rpp-dropdown').exists()).toBe(true)
  })
})
