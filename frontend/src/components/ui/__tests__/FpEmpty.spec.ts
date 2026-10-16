import { describe, it, expect } from 'vitest'
import { mountFp } from './helpers'
import FpEmpty from '../FpEmpty.vue'

/**
 * T1-1 组件测试：FpEmpty 空态。
 *
 * 空态是运维面板的高频界面（无容器 / 无备份 / 无日志），此处锁定三件事：
 * 默认图标可被覆盖、title/description 为空时不渲染空段落、action 插槽
 * 仅在提供时出现（避免空占位撑高布局）。
 */
describe('FpEmpty 空态', () => {
  it('默认图标 oi oi-inbox（openicons 唯一图标源）', () => {
    const w = mountFp(FpEmpty)
    expect(w.find('.fp-empty__icon i').classes()).toContain('oi')
    expect(w.find('.fp-empty__icon i').classes()).toContain('oi-inbox')
  })

  it('title / description 渲染为独立段落', () => {
    const w = mountFp(FpEmpty, { props: { title: '暂无备份', description: '创建第一个备份任务' } })
    expect(w.find('.fp-empty__title').text()).toBe('暂无备份')
    expect(w.find('.fp-empty__desc').text()).toBe('创建第一个备份任务')
  })

  it('title / description 缺省：不渲染空段落（不留占位空白）', () => {
    const w = mountFp(FpEmpty)
    expect(w.find('.fp-empty__title').exists()).toBe(false)
    expect(w.find('.fp-empty__desc').exists()).toBe(false)
    // 图标仍渲染，空态不至于变成一片空白
    expect(w.find('.fp-empty__icon').exists()).toBe(true)
  })

  it('icon 可覆盖（按业务场景换语义图标）', () => {
    const w = mountFp(FpEmpty, { props: { icon: 'oi oi-database' } })
    expect(w.find('.fp-empty__icon i').classes()).toContain('oi-database')
  })

  it('action 插槽：提供时渲染动作区，缺省时不渲染', () => {
    const withAction = mountFp(FpEmpty, { slots: { action: '<button class="go">新建</button>' } })
    expect(withAction.find('.fp-empty__action .go').text()).toBe('新建')

    const without = mountFp(FpEmpty)
    expect(without.find('.fp-empty__action').exists()).toBe(false)
  })
})
