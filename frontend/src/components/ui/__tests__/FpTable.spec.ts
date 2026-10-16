import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { h, defineComponent, type Component } from 'vue'
import FpTable from '../FpTable.vue'
import FpColumn from '../FpColumn.vue'
import Column from 'openvue/column'
import { mountFp } from './helpers'

const rows = [
  { name: '站点A', domain: 'a.com' },
  { name: '站点B', domain: 'b.com' },
]

const mkHost = (colComp: Component) =>
  defineComponent({
    render() {
      return h(FpTable as unknown as Component, { rows, paginator: false }, () => [
        h(colComp, { field: 'name', header: '名称' }),
        h(colComp, { field: 'domain', header: '域名' }),
      ])
    },
  })

describe('FpTable 列渲染验证', () => {
  it('FpColumn 必须与 OpenVue Column 同一组件（透明再导出）', () => {
    expect(FpColumn).toBe(Column)
  })
  it('裸 Column：单元格应渲染字段值', () => {
    const w = mount(mkHost(Column))
    expect(w.text()).toContain('站点A')
  })
  it('FpColumn 封装：单元格应渲染字段值', () => {
    const w = mount(mkHost(FpColumn as unknown as Component))
    expect(w.text()).toContain('站点A')
  })
})

/**
 * T1-1 扩展：表格的装配契约（列声明、空态、加载态、分页事件）。
 *
 * 「列收集」由 R-1 的同一性断言看住；这里补的是外围：不给行数据时表格不能
 * 塌成空白、加载态要有可见反馈、翻页只通过 `update:first` 上报（受控分页，
 * 服务端分页的现实约束）。
 */
const mountTable = (props: Record<string, unknown> = {}, slots: Record<string, string> = {}) =>
  mountFp(FpTable as unknown as Component, {
    props: { rows: [], paginator: false, ...props },
    slots: {
      default: () => h(Column, { field: 'name', header: '名称' }) as unknown as string,
      ...slots,
    },
  })

describe('FpTable 装配契约（T1-1）', () => {
  it('rows 非空：渲染表头与全部数据行', () => {
    const w = mountTable({ rows })
    expect(w.find('.p-datatable-thead').text()).toContain('名称')
    // 两行数据都落到表体（默认无虚拟滚动）
    expect(w.findAll('.p-datatable-tbody [data-p-index]')).toHaveLength(2)
    expect(w.text()).toContain('站点A')
    expect(w.text()).toContain('站点B')
  })

  it('rows 为空：渲染空态插槽（FpEmpty），不出现数据行', () => {
    const w = mountTable({ rows: [], emptyText: '暂无网站' })
    expect(w.find('.fp-empty').exists()).toBe(true)
    expect(w.find('.fp-empty__title').text()).toBe('暂无网站')
    expect(w.findAll('.p-datatable-tbody [data-p-index]')).toHaveLength(0)
  })

  it('loading：渲染加载提示而非数据行（首屏不给半成品表）', () => {
    const w = mountTable({ rows, loading: true, loadingText: '加载中' })
    expect(w.find('.fp-table__loading').exists()).toBe(true)
    expect(w.find('.fp-table__loading').text()).toContain('加载中')
  })

  it('loading 缺省：不渲染加载层', () => {
    const w = mountTable({ rows })
    expect(w.find('.fp-table__loading').exists()).toBe(false)
  })

  it('paginator=true 且行数超页：渲染分页条', () => {
    const many = Array.from({ length: 25 }, (_, i) => ({ name: `站点${i}`, domain: `${i}.com` }))
    const w = mountTable({ rows: many, paginator: true, rowsPerPage: 10 })
    expect(w.find('.p-paginator').exists()).toBe(true)
  })

  it('翻页发出 update:first（受控分页：状态由父组件持有）', async () => {
    const many = Array.from({ length: 25 }, (_, i) => ({ name: `站点${i}`, domain: `${i}.com` }))
    const w = mountTable({ rows: many, paginator: true, rowsPerPage: 10 })
    await w.find('.p-paginator-next').trigger('click')
    expect(w.emitted('update:first')?.at(-1)).toEqual([10])
  })

  it('paginator=false：不渲染分页条（客户端全量渲染）', () => {
    const w = mountTable({ rows, paginator: false })
    expect(w.find('.p-paginator').exists()).toBe(false)
  })

  it('virtual=true：自动禁用分页（虚拟滚动与分页互斥）', () => {
    const many = Array.from({ length: 200 }, (_, i) => ({ name: `站点${i}`, domain: `${i}.com` }))
    const w = mountTable({ rows: many, virtual: true, paginator: true })
    expect(w.find('.p-paginator').exists()).toBe(false)
  })

  it('stripedRows / scrollable 透传到底层表格', () => {
    const w = mountTable({ rows, stripedRows: true, scrollable: true, scrollHeight: '300px' })
    expect(w.find('.p-datatable-striped').exists()).toBe(true)
  })
})
