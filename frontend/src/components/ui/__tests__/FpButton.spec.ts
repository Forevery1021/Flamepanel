import { describe, it, expect } from 'vitest'
import { mountFp } from './helpers'
import FpButton from '../FpButton.vue'

/**
 * T1-1 组件测试：FpButton 语义变体 → OpenVue severity 的映射。
 *
 * 映射表是 Fp 封装层的核心契约（views 只认 `primary/secondary/danger/success/
 * warning/ghost/link`），底层库换版时错误映射不会报错、只会样式走样，故需要
 * 单测锁定。
 */
describe('FpButton 语义变体映射', () => {
  it('默认 primary：渲染 label 且非 plain/outlined/text', () => {
    const w = mountFp(FpButton, { props: { label: '保存' } })
    const btn = w.find('button')
    expect(btn.exists()).toBe(true)
    expect(btn.text()).toBe('保存')
    expect(btn.classes()).toContain('p-button')
    expect(btn.classes()).not.toContain('p-button-plain')
    expect(btn.classes()).not.toContain('p-button-outlined')
    expect(btn.classes()).not.toContain('p-button-text')
  })

  it('ghost → plain + outlined（次级实心，非描边语义）', () => {
    const w = mountFp(FpButton, { props: { variant: 'ghost', label: 'x' } })
    const classes = w.find('button').classes()
    expect(classes).toContain('p-button-plain')
    expect(classes).toContain('p-button-outlined')
  })

  it('link → text 样式', () => {
    const w = mountFp(FpButton, { props: { variant: 'link', label: 'x' } })
    expect(w.find('button').classes()).toContain('p-button-text')
  })

  it('danger → 危险色样式（删除等破坏性操作）', () => {
    const w = mountFp(FpButton, { props: { variant: 'danger', label: '删除' } })
    expect(w.find('button').classes().join(' ')).toContain('danger')
  })

  it('默认 type=button —— 避免置于表单内时误触提交', () => {
    const w = mountFp(FpButton, { props: { label: 'x' } })
    expect(w.find('button').attributes('type')).toBe('button')
  })

  it('loading 态：按钮进入加载样式', () => {
    const w = mountFp(FpButton, { props: { label: 'x', loading: true } })
    expect(w.find('button').classes().join(' ')).toContain('loading')
  })

  it('disabled 透传到底层按钮', () => {
    const w = mountFp(FpButton, { props: { label: 'x', disabled: true } })
    expect(w.find('button').attributes('disabled')).toBeDefined()
  })

  it('slot 内容优先于 label', () => {
    const w = mountFp(FpButton, { props: { label: 'label' }, slots: { default: 'slot 文案' } })
    expect(w.text()).toContain('slot 文案')
  })
})
