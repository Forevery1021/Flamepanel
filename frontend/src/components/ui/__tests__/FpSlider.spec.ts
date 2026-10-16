import { describe, it, expect } from 'vitest'
import { mountFp } from './helpers'
import FpSlider from '../FpSlider.vue'
import FpSelectButton from '../FpSelectButton.vue'

/**
 * T1-1 组件测试：FpSlider 滑块 / FpSelectButton 单选按钮组。
 *
 * FpSlider 的关键归一逻辑：底层可回传标量或数组（多手柄模式），封装层必须
 * 收敛为单值 number——否则 `v-model` 绑定的数值会突然变成数组，后续算术
 * 运算静默产出 NaN（这一层归一正是封装价值所在）。
 */
describe('FpSlider 滑块', () => {
  it('默认 value=0，渲染滑块控件', () => {
    const w = mountFp(FpSlider, {})
    expect(w.find('.p-slider').exists()).toBe(true)
  })

  it('modelValue 反映到底层（受控）', () => {
    const w = mountFp(FpSlider, { props: { modelValue: 40 } })
    expect(w.html()).toBeTruthy()
    expect(w.find('.p-slider').exists()).toBe(true)
  })

  it('底层回传标量：原样上报', async () => {
    const w = mountFp(FpSlider, { props: { modelValue: 0 } })
    await w.findComponent({ name: 'Slider' }).vm.$emit('update:modelValue', 55)
    expect(w.emitted('update:modelValue')?.at(-1)).toEqual([55])
  })

  it('【归一】底层回传数组：收敛为数组首元素（多手柄 → 单值）', async () => {
    const w = mountFp(FpSlider, { props: { modelValue: 0 } })
    await w.findComponent({ name: 'Slider' }).vm.$emit('update:modelValue', [30, 70])
    const payload = w.emitted('update:modelValue')?.at(-1)?.[0]
    expect(payload).toBe(30)
    expect(Array.isArray(payload)).toBe(false)
  })

  it('【归一】数组为空：回落 0（不产生 undefined）', async () => {
    // 初值取非 0，确保归一结果与当前值不同、能观察到 emit（defineModel 对同值不重复上报）
    const w = mountFp(FpSlider, { props: { modelValue: 5 } })
    await w.findComponent({ name: 'Slider' }).vm.$emit('update:modelValue', [])
    const payload = w.emitted('update:modelValue')?.at(-1)?.[0]
    expect(payload).toBe(0)
    expect(typeof payload).toBe('number')
  })
})

describe('FpSelectButton 单选按钮组', () => {
  // 与 RuntimeView 同款用法：显式给 option-label / option-value，否则底层会把
  // 整个选项对象当显示名（Vue 在开发态还会打出 prop 类型告警）
  const opts = [
    { label: 'PHP', value: 'php' },
    { label: 'Node', value: 'node' },
  ]

  it('渲染按钮组容器与各项文案', () => {
    const w = mountFp(FpSelectButton, {
      props: { modelValue: 'php', options: opts, optionLabel: 'label', optionValue: 'value' },
    })
    expect(w.find('.p-selectbutton').exists()).toBe(true)
    expect(w.text()).toContain('PHP')
    expect(w.text()).toContain('Node')
  })

  it('选择项：发出 update:modelValue=该项值', async () => {
    const w = mountFp(FpSelectButton, {
      props: { modelValue: 'php', options: opts, optionLabel: 'label', optionValue: 'value' },
    })
    const btns = w.findAll('button')
    expect(btns.length).toBeGreaterThanOrEqual(2)
    await btns[1]!.trigger('click')
    expect(w.emitted('update:modelValue')?.at(-1)).toEqual(['node'])
  })

  it('allowEmpty=false 透传（运行时/主题切换不允许取消选择）', () => {
    const w = mountFp(FpSelectButton, {
      props: {
        modelValue: 'php',
        options: opts,
        optionLabel: 'label',
        optionValue: 'value',
        allowEmpty: false,
      },
    })
    expect(w.find('.p-selectbutton').exists()).toBe(true)
  })

  it('空值归一为 ""（父组件不必处理 undefined）', async () => {
    const w = mountFp(FpSelectButton, { props: { modelValue: 'a' } })
    await w.findComponent({ name: 'SelectButton' }).vm.$emit('update:modelValue', undefined)
    expect(w.emitted('update:modelValue')?.at(-1)).toEqual([''])
  })
})
