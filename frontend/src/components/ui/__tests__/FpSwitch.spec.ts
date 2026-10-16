import { describe, it, expect } from 'vitest'
import { mountFp } from './helpers'
import FpSwitch from '../FpSwitch.vue'

/**
 * T1-1 组件测试：FpSwitch 开关。
 *
 * 开关被用于「启用/停用」这类即时生效的操作（证书自动续签、厂商账号启停），
 * 契约重点是：受控（只通过 v-model 上报，不自行改状态）、disabled 时不可切换。
 */
const inputOf = (w: ReturnType<typeof mountFp>) => w.find('input[type="checkbox"]')

describe('FpSwitch 开关', () => {
  it('modelValue=true：渲染为已开启态', () => {
    const w = mountFp(FpSwitch, { props: { modelValue: true } })
    expect(w.find('.p-toggleswitch').classes()).toContain('p-toggleswitch-checked')
    expect(inputOf(w).attributes('aria-checked')).toBe('true')
  })

  it('modelValue=false：渲染为关闭态', () => {
    const w = mountFp(FpSwitch, { props: { modelValue: false } })
    expect(w.find('.p-toggleswitch').classes()).not.toContain('p-toggleswitch-checked')
    expect(inputOf(w).attributes('aria-checked')).toBe('false')
  })

  it('切换：发出 update:modelValue=true（父组件是唯一状态来源）', async () => {
    const w = mountFp(FpSwitch, { props: { modelValue: false } })
    await inputOf(w).setValue(true)
    expect(w.emitted('update:modelValue')?.at(-1)).toEqual([true])
  })

  it('父组件回写后 UI 同步（v-model 闭环）', async () => {
    const w = mountFp(FpSwitch, { props: { modelValue: false } })
    await inputOf(w).setValue(true)
    await w.setProps({ modelValue: true })
    expect(w.find('.p-toggleswitch').classes()).toContain('p-toggleswitch-checked')
  })

  it('禁用态：input 带 disabled', () => {
    const w = mountFp(FpSwitch, { props: { modelValue: false, disabled: true } })
    expect(inputOf(w).attributes('disabled')).toBeDefined()
  })

  it('关闭后再次开启：状态翻转正确', async () => {
    const w = mountFp(FpSwitch, { props: { modelValue: true } })
    await inputOf(w).setValue(false)
    expect(w.emitted('update:modelValue')?.at(-1)).toEqual([false])
  })

  it('label：渲染标签并与 input 关联（点击标签可切换）', () => {
    const w = mountFp(FpSwitch, { props: { modelValue: false, label: '启用自动备份' } })
    const label = w.find('.fp-switch__label')
    expect(label.text()).toBe('启用自动备份')
    expect(label.attributes('for')).toBe(inputOf(w).attributes('id'))
    expect(w.find('.fp-switch').classes()).toContain('fp-switch--labeled')
  })

  it('hint：渲染辅助说明；缺省不渲染标签/说明节点', () => {
    const w = mountFp(FpSwitch, { props: { modelValue: false, hint: '每日 03:00 执行' } })
    expect(w.find('.fp-switch__hint').text()).toBe('每日 03:00 执行')
    expect(w.find('.fp-switch__label').exists()).toBe(false)
    const bare = mountFp(FpSwitch, { props: { modelValue: false } })
    expect(bare.find('.fp-switch__hint').exists()).toBe(false)
  })
})
