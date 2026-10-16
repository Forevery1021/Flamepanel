import { describe, it, expect } from 'vitest'
import { mountFp } from './helpers'
import FpFormField from '../FpFormField.vue'

/**
 * T1-1 组件测试：FpFormField 表单字段容器。
 *
 * 容器本身不处理数据，价值全在「label / hint / error 三者的互斥与关联」：
 * 错误出现时应顶掉 hint（一处空间只表达一件事），且 label 的 for 必须指向
 * 真实存在的控件 id，否则点击标签无法聚焦输入框。
 */
const mountField = (props: Record<string, unknown> = {}, slots: Record<string, string> = {}) =>
  mountFp(FpFormField, { props, slots: { default: '<input id="ctl" />', ...slots } })

describe('FpFormField 字段容器', () => {
  it('label + hint：渲染标签与提示，且不出现错误态', () => {
    const w = mountField({ label: '数据库名', hint: '仅支持小写字母' })
    expect(w.find('.fp-form-field__label').text()).toBe('数据库名')
    expect(w.find('.fp-form-field__hint').text()).toBe('仅支持小写字母')
    expect(w.find('.fp-form-field__error').exists()).toBe(false)
    expect(w.find('.fp-form-field').classes()).not.toContain('fp-form-field-invalid')
  })

  it('error 非空：容器进入非法态并渲染错误文案', () => {
    const w = mountField({ label: '数据库名', error: '已存在同名数据库' })
    expect(w.find('.fp-form-field').classes()).toContain('fp-form-field-invalid')
    expect(w.find('.fp-form-field__error').text()).toBe('已存在同名数据库')
  })

  it('【互斥】error 出现时 hint 让位（同一位置不做双重表达）', () => {
    const w = mountField({ label: 'x', hint: '提示', error: '错误' })
    expect(w.find('.fp-form-field__error').exists()).toBe(true)
    expect(w.find('.fp-form-field__hint').exists()).toBe(false)
  })

  it('error 为空串：视为无错误（业务侧常用空串表达「校验通过」）', () => {
    const w = mountField({ label: 'x', error: '', hint: '提示' })
    expect(w.find('.fp-form-field__error').exists()).toBe(false)
    expect(w.find('.fp-form-field__hint').exists()).toBe(true)
  })

  it('无 label：不渲染 label 元素但保留插槽', () => {
    const w = mountField({}, { default: '<input class="plain" />' })
    expect(w.find('.fp-form-field__label').exists()).toBe(false)
    expect(w.find('.plain').exists()).toBe(true)
  })

  it('label 的 for 与 error 的 id 均指向稳定生成值（同一次挂载内不漂移）', () => {
    const w = mountField({ label: '站点', error: '必填' })
    const forId = w.find('.fp-form-field__label').attributes('for')
    const errorId = w.find('.fp-form-field__error').attributes('id')
    expect(forId).toBeTruthy()
    // 错误描述 id 以 label 的 for 为前缀，便于 aria-describedby 关联
    expect(errorId).toBe(`${forId}-error`)
  })

  it('slot 内容不被容器改写（透传自定义控件）', () => {
    const w = mountField({}, { default: '<textarea class="custom"></textarea>' })
    expect(w.find('.custom').exists()).toBe(true)
  })
})
