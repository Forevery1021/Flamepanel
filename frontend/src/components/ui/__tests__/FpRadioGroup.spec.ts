import { describe, it, expect } from 'vitest'
import { h } from 'vue'
import { mountFp } from './helpers'
import FpRadioGroup from '../FpRadioGroup.vue'
import FpRadioOption from '../FpRadioOption.vue'

/**
 * T1-1 组件测试：FpRadioGroup / FpRadioOption 单选组。
 *
 * 两条装配路径都要可用：`options` 声明式（数据驱动，如数据库类型选择）与
 * 默认插槽自定义（需要更丰富排版时）。选中值经 `update:modelValue` 上报，
 * 空值归一为 ''（避免父组件拿到 undefined 后判空分支失控）。
 */
const options = [
  { label: 'MySQL', value: 'mysql' },
  { label: 'PostgreSQL', value: 'postgres' },
]

describe('FpRadioGroup 单选组', () => {
  it('options 声明式：每项渲染一个单选按钮（label 落在 label 属性上）', () => {
    const w = mountFp(FpRadioGroup, { props: { modelValue: 'mysql', options } })
    expect(w.find('.p-radiobutton-group').exists()).toBe(true)
    const radios = w.findAll('.p-radiobutton')
    expect(radios).toHaveLength(2)
    // 底层 RadioButton 把 label 写进元素属性（视觉文案由调用方渲染），非文本节点
    expect(radios[0]!.attributes('label')).toBe('MySQL')
    expect(radios[1]!.attributes('label')).toBe('PostgreSQL')
  })

  it('选中项带 data-p-checked=true', () => {
    const w = mountFp(FpRadioGroup, { props: { modelValue: 'postgres', options } })
    const checked = w.findAll('[data-p-checked="true"]')
    expect(checked).toHaveLength(1)
  })

  it('点击另一项：发出 update:modelValue=该项值', async () => {
    const w = mountFp(FpRadioGroup, { props: { modelValue: 'mysql', options } })
    const radios = w.findAll('input[type="radio"]')
    expect(radios.length).toBeGreaterThanOrEqual(2)
    await radios[1]!.setValue(true)
    expect(w.emitted('update:modelValue')?.at(-1)).toEqual(['postgres'])
  })

  it('【常用路径】默认插槽 + FpRadioOption + 外部 label（WebServersView 同款）', () => {
    // 业务用 `inputId` 把单选按钮与外部 <label for> 绑定，label 由调用方渲染
    const w = mountFp(FpRadioGroup, {
      props: { modelValue: 'fastcgi' },
      slots: {
        default: () =>
          h(FpRadioOption, { value: 'fastcgi', inputId: 'preset-fastcgi' }) as unknown as string,
      },
    })
    const input = w.find('input[type="radio"]')
    expect(input.exists()).toBe(true)
    expect(input.attributes('id')).toBe('preset-fastcgi')
    expect(input.attributes('value')).toBe('fastcgi')
  })

  it('options 缺省：不渲染任何单选按钮（空数据安全）', () => {
    const w = mountFp(FpRadioGroup, { props: { modelValue: 'x' } })
    expect(w.findAll('input[type="radio"]')).toHaveLength(0)
  })

  it('optionLabel / optionValue 可指向自定义字段名', () => {
    const w = mountFp(FpRadioGroup, {
      props: {
        modelValue: 'v1',
        options: [{ text: '文本一', id: 'v1' }],
        optionLabel: 'text',
        optionValue: 'id',
      },
    })
    expect(w.find('.p-radiobutton').attributes('label')).toBe('文本一')
    expect(w.find('input').attributes('value')).toBe('v1')
  })
})

describe('FpRadioOption 单选按钮', () => {
  it('渲染为可点击的单选按钮（自定义组合用）', () => {
    const w = mountFp(FpRadioOption, { props: { value: 'x' } })
    expect(w.find('input[type="radio"]').exists()).toBe(true)
  })

  it('inputId 透传到底层 input（label 可关联）', () => {
    const w = mountFp(FpRadioOption, { props: { value: 'x' } })
    expect(w.find('input')?.attributes('id')).toBeDefined()
  })
})
