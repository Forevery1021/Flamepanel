import { describe, it, expect } from 'vitest'
import { mountFp } from './helpers'
import FpMultiSelect from '../FpMultiSelect.vue'

/** W4-a：多选语义（旧版 `FpSelect multiple` 被底层单选组件静默忽略） */
const options = [
  { label: 'Web', value: 'web' },
  { label: '应用', value: 'apps' },
]

describe('FpMultiSelect', () => {
  it('渲染 OpenVue MultiSelect，已选值以数组传入', () => {
    const w = mountFp(FpMultiSelect, {
      props: { modelValue: ['web'], options, optionLabel: 'label', optionValue: 'value' },
    })
    expect(w.find('.p-multiselect').exists()).toBe(true)
    expect(w.text()).toContain('Web')
  })

  it('label + inputId：标签 for 指向内部输入', () => {
    const w = mountFp(FpMultiSelect, {
      props: { modelValue: [], options, label: '隐藏分组', inputId: 'hide-menu' },
    })
    expect(w.find('label').attributes('for')).toBe('hide-menu')
    expect(w.find('#hide-menu').exists()).toBe(true)
  })

  it('清空时回写空数组而不是 undefined', async () => {
    const w = mountFp(FpMultiSelect, { props: { modelValue: ['web'], options } })
    const inner = w.findComponent({ name: 'MultiSelect' })
    inner.vm.$emit('update:modelValue', null)
    expect(w.emitted('update:modelValue')?.[0]).toEqual([[]])
  })
})
