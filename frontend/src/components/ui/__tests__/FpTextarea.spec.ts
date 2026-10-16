import { describe, it, expect } from 'vitest'
import { mountFp } from './helpers'
import FpTextarea from '../FpTextarea.vue'

/**
 * T1-1 组件测试：FpTextarea 多行文本。
 *
 * 与 FpInput 同属字段族，字段态契约（error → 视觉非法态 + aria 关联）必须一致；
 * 差异是它没有 label（多行输入在业务里由外层 FpFormField 提供标签）。
 */
describe('FpTextarea 多行文本', () => {
  it('v-model：输入触发 update:modelValue', async () => {
    const w = mountFp(FpTextarea, { props: { modelValue: '' } })
    await w.find('textarea').setValue('第一行\n第二行')
    expect(w.emitted('update:modelValue')?.at(-1)).toEqual(['第一行\n第二行'])
  })

  it('error 非空：aria-invalid + aria-describedby 指向真实存在的错误节点', () => {
    const w = mountFp(FpTextarea, { props: { modelValue: '', error: '内容过长' } })
    const ta = w.find('textarea')
    expect(ta.attributes('aria-invalid')).toBe('true')
    expect(w.find(`#${ta.attributes('aria-describedby')}`).text()).toBe('内容过长')
  })

  it('error 为空：不渲染错误节点、不标记 aria-invalid', () => {
    const w = mountFp(FpTextarea, { props: { modelValue: '' } })
    expect(w.find('.fp-textarea-error').exists()).toBe(false)
    expect(w.find('textarea').attributes('aria-invalid')).toBeUndefined()
  })

  it('invalid 手动置非法态：无文案也标记', () => {
    const w = mountFp(FpTextarea, { props: { modelValue: '', invalid: true } })
    expect(w.find('textarea').attributes('aria-invalid')).toBe('true')
  })

  it('$attrs 透传：rows / placeholder 落到底层 textarea', () => {
    const w = mountFp(FpTextarea, {
      props: { modelValue: '' },
      attrs: { rows: 6, placeholder: '描述' },
    })
    expect(w.find('textarea').attributes('rows')).toBe('6')
    expect(w.find('textarea').attributes('placeholder')).toBe('描述')
  })

  it('父组件回写后同步显示', async () => {
    const w = mountFp(FpTextarea, { props: { modelValue: 'a' } })
    await w.setProps({ modelValue: 'b' })
    expect((w.find('textarea').element as HTMLTextAreaElement).value).toBe('b')
  })

  it('label / hint：标签与 textarea 通过 for/id 关联；有 error 时 hint 让位', async () => {
    const w = mountFp(FpTextarea, { props: { modelValue: '', label: '命令', hint: '支持多行' } })
    expect(w.find('.fp-field__label').text()).toBe('命令')
    expect(w.find('.fp-field__label').attributes('for')).toBe(w.find('textarea').attributes('id'))
    expect(w.find('.fp-field__hint').text()).toBe('支持多行')
    await w.setProps({ error: '必填' })
    expect(w.find('.fp-field__hint').exists()).toBe(false)
    expect(w.find('.fp-textarea-error').text()).toBe('必填')
  })
})
