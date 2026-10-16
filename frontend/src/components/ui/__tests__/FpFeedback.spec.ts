import { describe, it, expect } from 'vitest'
import { mountFp } from './helpers'
import FpInlineMessage from '../FpInlineMessage.vue'
import FpProgress from '../FpProgress.vue'
import FpSkeleton from '../FpSkeleton.vue'
import FpChip from '../FpChip.vue'
import FpDivider from '../FpDivider.vue'
import FpBreadcrumb from '../FpBreadcrumb.vue'
import FpButtonLink from '../FpButtonLink.vue'

/**
 * T1-1 组件测试：轻量反馈与结构件（无状态、纯透传封装）。
 *
 * 这组成员本身逻辑极薄，测试价值不在「实现」而在**契约**：severity/等级映射、
 * 无障碍角色、透传字段是否真的落到 DOM。封装层出错时页面不报错、只是语义错位，
 * 靠人工 review 很难兜住。
 */
describe('FpInlineMessage 行内提示', () => {
  it('默认 info 等级，渲染 role=alert', () => {
    const w = mountFp(FpInlineMessage, { slots: { default: '提示内容' } })
    expect(w.text()).toContain('提示内容')
    expect(w.find('[role="alert"]').exists()).toBe(true)
    expect(w.html()).toContain('p-inlinemessage-info')
  })

  it('severity=error：渲染错误等级（配置错误高亮）', () => {
    const w = mountFp(FpInlineMessage, { props: { severity: 'error' }, slots: { default: 'x' } })
    expect(w.html()).toContain('p-inlinemessage-error')
  })

  it('severity=warn / success 各自映射', () => {
    expect(mountFp(FpInlineMessage, { props: { severity: 'warn' } }).html()).toContain('warn')
    expect(mountFp(FpInlineMessage, { props: { severity: 'success' } }).html()).toContain('success')
  })
})

describe('FpProgress 进度条', () => {
  it('value 反映到 aria-valuenow（无障碍可读）', () => {
    const w = mountFp(FpProgress, { props: { value: 62 } })
    expect(w.find('[role="progressbar"]').attributes('aria-valuenow')).toBe('62')
  })

  it('默认 value=0（未开始时也有确定值，不塌成空条）', () => {
    const w = mountFp(FpProgress, {})
    expect(w.find('[role="progressbar"]').attributes('aria-valuenow')).toBe('0')
  })

  it('value=100：满进度', () => {
    const w = mountFp(FpProgress, { props: { value: 100 } })
    expect(w.find('[role="progressbar"]').attributes('aria-valuenow')).toBe('100')
  })
})

describe('FpSkeleton 骨架屏', () => {
  it('渲染骨架元素且对辅助技术隐藏（加载占位不是内容）', () => {
    const w = mountFp(FpSkeleton, {})
    expect(w.find('.fp-skeleton').exists()).toBe(true)
    expect(w.find('.fp-skeleton').attributes('aria-hidden')).toBe('true')
  })
})

describe('FpChip 标签块', () => {
  it('渲染 label', () => {
    const w = mountFp(FpChip, { props: { label: 'php:8.3' } })
    expect(w.html()).toContain('php:8.3')
  })

  it('removable 透传（可移除标记）', () => {
    const w = mountFp(FpChip, { props: { label: 'x', removable: true } })
    expect(w.find('.p-chip').attributes('data-p')).toContain('removable')
  })
})

describe('FpDivider 分隔线', () => {
  it('渲染为 separator 角色（结构语义）', () => {
    const w = mountFp(FpDivider, {})
    expect(w.find('[role="separator"]').exists()).toBe(true)
  })
})

describe('FpBreadcrumb 面包屑', () => {
  it('items 渲染为面包屑条目', () => {
    const w = mountFp(FpBreadcrumb, { props: { items: [{ label: '首页' }, { label: '网站' }] } })
    expect(w.text()).toContain('首页')
    expect(w.text()).toContain('网站')
  })

  it('items 缺省：不崩，容器仍渲染', () => {
    const w = mountFp(FpBreadcrumb, {})
    expect(w.find('.fp-breadcrumb').exists()).toBe(true)
  })
})

describe('FpButtonLink 链接按钮', () => {
  it('默认 text 样式（视觉上退为链接）', () => {
    const w = mountFp(FpButtonLink, { props: { label: '取消' } })
    expect(w.find('button').classes()).toContain('p-button-text')
    expect(w.text()).toBe('取消')
  })

  it('size=large 透传', () => {
    const w = mountFp(FpButtonLink, { props: { label: 'x', size: 'large' } })
    expect(w.find('button').classes().join(' ')).toContain('p-button')
  })
})
