import { describe, it, expect } from 'vitest'
import { mountFp } from './helpers'
import FpCard from '../FpCard.vue'

/**
 * T1-1 组件测试：FpCard 品牌卡片。
 *
 * 仪表盘与各详情页的容器组件：标题栏带品牌色竖条（视觉识别的一部分，用
 * aria-hidden 标记为装饰元素，避免读屏重复念出）。extra 插槽是标题右侧的
 * 操作位（如「刷新」「查看全部」）。
 */
describe('FpCard 卡片', () => {
  it('title 渲染在标题栏，且带装饰竖条', () => {
    const w = mountFp(FpCard, { props: { title: '系统负载' } })
    expect(w.find('.fp-card-title__text').text()).toBe('系统负载')
    const bar = w.find('.fp-card-title__bar')
    expect(bar.exists()).toBe(true)
    // 装饰元素对辅助技术隐藏
    expect(bar.attributes('aria-hidden')).toBe('true')
  })

  it('【回归·已修复】默认插槽内容渲染进 p-card-content', () => {
    // 历史缺陷（T1-1 发现）：底层 openvue/card 只把命名插槽 `content` 渲染进
    // p-card-content，而 FpCard 的 `<slot />` 落进 Card 的**默认**插槽 → 无处渲染，
    // 内容被静默丢弃且无报错。影响 RuntimeView：卡片主体（安装路径 + 安装/设为
    // 默认/卸载按钮）整体不可见。
    // 修复：FpCard 内改为 `<template #content><slot /></template>`，本条断言随之翻转。
    const w = mountFp(FpCard, {
      props: { title: 'x' },
      slots: { default: '<div class="body">内容</div>' },
    })
    const content = w.find('.p-card-content')
    expect(content.exists()).toBe(true)
    expect(content.find('.body').exists()).toBe(true)
    expect(content.find('.body').text()).toBe('内容')
  })

  it('默认插槽正文与标题、extra、footer 同屏共存（不串位）', () => {
    const w = mountFp(FpCard, {
      props: { title: '系统负载' },
      slots: {
        default: '<div class="body">正文</div>',
        extra: '<button class="more">更多</button>',
        footer: '<div class="ft">脚注</div>',
      },
    })
    expect(w.find('.fp-card-title__text').text()).toBe('系统负载')
    expect(w.find('.p-card-content .body').text()).toBe('正文')
    expect(w.find('.fp-card-title__extra .more').text()).toBe('更多')
    expect(w.find('.p-card-footer .ft').text()).toBe('脚注')
    // 正文不在标题栏/底栏内（避免命名插槽转发串位）
    expect(w.find('.fp-card-title .body').exists()).toBe(false)
  })

  it('默认插槽缺省：content 区域仍渲染（空卡片结构一致）', () => {
    const w = mountFp(FpCard, { props: { title: 'x' } })
    expect(w.find('.p-card-content').exists()).toBe(true)
  })

  it('extra 插槽：渲染标题右侧操作位，缺省时不渲染', () => {
    const withExtra = mountFp(FpCard, {
      props: { title: 'x' },
      slots: { extra: '<button class="more">更多</button>' },
    })
    expect(withExtra.find('.fp-card-title__extra .more').text()).toBe('更多')

    const without = mountFp(FpCard, { props: { title: 'x' } })
    expect(without.find('.fp-card-title__extra').exists()).toBe(false)
  })

  it('footer 插槽：提供时渲染底栏，缺省时不渲染', () => {
    const withFooter = mountFp(FpCard, {
      props: { title: 'x' },
      slots: { footer: '<div class="ft">脚注</div>' },
    })
    expect(withFooter.find('.ft').text()).toBe('脚注')

    const without = mountFp(FpCard, { props: { title: 'x' } })
    expect(without.find('.ft').exists()).toBe(false)
  })

  it('title 缺省：标题栏仍渲染（保持卡片结构一致）', () => {
    const w = mountFp(FpCard, {})
    expect(w.find('.fp-card').exists()).toBe(true)
    expect(w.find('.fp-card-title__text').text()).toBe('')
  })
})
