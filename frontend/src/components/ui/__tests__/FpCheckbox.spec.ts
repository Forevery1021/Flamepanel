import { describe, it, expect } from 'vitest'
import { mountFp } from './helpers'
import FpCheckbox from '../FpCheckbox.vue'

/**
 * T1-1 组件测试：FpCheckbox 复选框。
 *
 * 与 FpSwitch 的差异在于它额外发出 `change` 事件——业务侧用它做「勾选即触发
 * 批量操作」的即时反馈，故两条事件通道都要锁住。
 *
 * C-3 回归：`change` 的载荷必须是**本次交互后的新值**（历史缺陷下恒为 `true`，
 * 取消勾选信号丢失，「取消完成」等反向操作失效）。
 */
describe('FpCheckbox 复选框', () => {
  it('modelValue=false：未勾选', () => {
    const w = mountFp(FpCheckbox, { props: { modelValue: false } })
    expect(w.find('.p-checkbox').classes()).not.toContain('p-checkbox-checked')
  })

  it('modelValue=true：已勾选（binary 模式）', () => {
    const w = mountFp(FpCheckbox, { props: { modelValue: true } })
    expect(w.find('.p-checkbox').classes()).toContain('p-checkbox-checked')
    expect(w.find('input[type="checkbox"]').attributes('checked')).toBeDefined()
  })

  it('勾选：发出 update:modelValue 与 change 两个事件', async () => {
    const w = mountFp(FpCheckbox, { props: { modelValue: false } })
    await w.find('input').setValue(true)
    expect(w.emitted('update:modelValue')?.at(-1)).toEqual([true])
    expect(w.emitted('change')?.at(-1)).toEqual([true])
  })

  it('【C-3 回归】取消勾选：change 载荷为 false（历史缺陷恒为 true）', async () => {
    // 底层 `change` 事件带的是 DOM Event 对象而非新值，旧实现 `Boolean(event)` 恒真：
    // MemoList / Dashboard 待办的「取消完成」因此失效（永远传 done=true）。
    // 修复后以事件源 input 的 checked 为准，`change` 与 `update:modelValue` 同源。
    const w = mountFp(FpCheckbox, { props: { modelValue: true } })
    await w.find('input').setValue(false)
    expect(w.emitted('change')?.at(-1)).toEqual([false])
    // 两条通道一致：v-model 也是 false
    expect(w.emitted('update:modelValue')?.at(-1)).toEqual([false])
  })

  it('【C-3 回归】连续切换：change 载荷逐次跟随真实状态（true → false → true）', async () => {
    const w = mountFp(FpCheckbox, { props: { modelValue: false } })
    await w.find('input').setValue(true)
    await w.setProps({ modelValue: true })
    await w.find('input').setValue(false)
    await w.setProps({ modelValue: false })
    await w.find('input').setValue(true)
    expect(w.emitted('change')?.map((args) => args[0])).toEqual([true, false, true])
    expect(w.emitted('update:modelValue')?.map((args) => args[0])).toEqual([true, false, true])
  })

  it('父组件回写后 UI 同步（v-model 闭环）', async () => {
    const w = mountFp(FpCheckbox, { props: { modelValue: false } })
    await w.setProps({ modelValue: true })
    expect(w.find('.p-checkbox').classes()).toContain('p-checkbox-checked')
  })

  it('$attrs 透传（disabled 等约束）', () => {
    const w = mountFp(FpCheckbox, { props: { modelValue: false }, attrs: { disabled: true } })
    expect(w.find('input').attributes('disabled')).toBeDefined()
  })
})
