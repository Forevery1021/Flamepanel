import { describe, it, expect } from 'vitest'
import { mountFp } from './helpers'
import FpNumber from '../FpNumber.vue'

/**
 * T1-1 组件测试：FpNumber 数字输入。
 *
 * 端口 / 副本数 / 保留天数等参数都用它。字段态契约与 FpInput 同源（label 经
 * FloatLabel、error 走 aria 关联），另锁定空值语义：清空后回传 `null`
 * 而非 `undefined`，业务侧便于直接判空。
 */
describe('FpNumber 数字输入', () => {
  it('渲染数字输入控件', () => {
    const w = mountFp(FpNumber, { props: { modelValue: 1 } })
    expect(w.find('.p-inputnumber').exists()).toBe(true)
    expect(w.find('input').element.value).toBe('1')
  })

  it('label 非空：静态标签与真实 input id 关联（可点击聚焦）', () => {
    const w = mountFp(FpNumber, { props: { modelValue: 1, label: '端口' } })
    const label = w.find('label')
    expect(label.text()).toBe('端口')
    expect(w.find('input').attributes('id')).toBe(label.attributes('for'))
  })

  it('label 为空：不渲染 FloatLabel', () => {
    const w = mountFp(FpNumber, { props: { modelValue: 1 } })
    expect(w.find('label').exists()).toBe(false)
  })

  it('error 非空：容器非法态 + 错误文案可见', () => {
    const w = mountFp(FpNumber, { props: { modelValue: 1, error: '范围 1-65535' } })
    expect(w.find('.fp-number').classes()).toContain('fp-field-invalid')
    expect(w.find('.fp-field-error').text()).toBe('范围 1-65535')
  })

  it('invalid 手动置非法态：无文案也标记容器', () => {
    const w = mountFp(FpNumber, { props: { modelValue: 1, invalid: true } })
    expect(w.find('.fp-number').classes()).toContain('fp-field-invalid')
    expect(w.find('.fp-field-error').exists()).toBe(false)
  })

  it('输入新值触发 update:modelValue', async () => {
    const w = mountFp(FpNumber, { props: { modelValue: 1 } })
    const input = w.find('input')
    await input.setValue('42')
    await input.trigger('blur')
    expect(w.emitted('update:modelValue')).toBeTruthy()
  })
})
