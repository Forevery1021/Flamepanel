import { describe, it, expect } from 'vitest'
import { nextTick } from 'vue'
import { mountFp } from './helpers'
import FpInput from '../FpInput.vue'

/**
 * T1-1 组件测试：FpInput 的字段态契约。
 *
 * 关注三件在业务里高频、出错又不会报的事：
 * 1. `error` 必须同时驱动「视觉非法态」与「无障碍属性」——只改颜色会让读屏
 *    用户拿不到错误信息；
 * 2. 错误文案与输入框经 `aria-describedby` 关联，且 id 真实存在；
 * 3. `v-model` 双向绑定与 `$attrs` 透传（placeholder/disabled 等）。
 */
const errorOf = (w: ReturnType<typeof mountFp>) => w.find('.fp-field-error')

describe('FpInput 输入框', () => {
  it('v-model：输入触发 update:modelValue', async () => {
    const w = mountFp(FpInput, { props: { modelValue: '' } })
    const input = w.find('input')
    await input.setValue('hello')
    expect(w.emitted('update:modelValue')?.at(-1)).toEqual(['hello'])
    expect(input.element.value).toBe('hello')
  })

  it('error 非空：容器进入非法态且渲染错误文案', () => {
    const w = mountFp(FpInput, { props: { modelValue: '', error: '名称不能为空' } })
    expect(w.find('.fp-field').classes()).toContain('fp-field-invalid')
    expect(errorOf(w).text()).toBe('名称不能为空')
  })

  it('【无障碍】error 同时写入 aria-invalid 与 aria-describedby', () => {
    const w = mountFp(FpInput, { props: { modelValue: '', error: '必填' } })
    const input = w.find('input')
    expect(input.attributes('aria-invalid')).toBe('true')
    const describedBy = input.attributes('aria-describedby')
    expect(describedBy).toBeTruthy()
    // 描述的目标元素必须真实存在（悬空 id 等于没有提示）
    expect(w.find(`#${describedBy}`).exists()).toBe(true)
  })

  it('invalid 手动置非法态：即使无文案也标记 aria-invalid', () => {
    const w = mountFp(FpInput, { props: { modelValue: '', invalid: true } })
    expect(w.find('.fp-field').classes()).toContain('fp-field-invalid')
    expect(w.find('input').attributes('aria-invalid')).toBe('true')
    expect(errorOf(w).exists()).toBe(false)
  })

  it('label 非空：静态标签与控件 id 关联， label 与输入框 id 关联', () => {
    const w = mountFp(FpInput, { props: { modelValue: '', label: '域名' } })
    const label = w.find('label')
    expect(label.exists()).toBe(true)
    expect(label.text()).toBe('域名')
    expect(w.find('input').attributes('id')).toBe(label.attributes('for'))
  })

  it('label 为空：不渲染标签（保持紧凑布局）', () => {
    const w = mountFp(FpInput, { props: { modelValue: '' } })
    expect(w.find('label').exists()).toBe(false)
  })

  it('align=right：追加右对齐类名（数值列对齐）', () => {
    const w = mountFp(FpInput, { props: { modelValue: '', align: 'right' } })
    expect(w.find('input').classes()).toContain('text-right')
  })

  it('$attrs 透传：placeholder / disabled 落到底层 input', () => {
    const w = mountFp(FpInput, {
      props: { modelValue: '', placeholder: '请输入' },
      attrs: { disabled: true },
    })
    expect(w.find('input').attributes('placeholder')).toBe('请输入')
    expect(w.find('input').attributes('disabled')).toBeDefined()
    expect(w.find('input').attributes('disabled')).toBe('')
  })

  it('父组件改写 modelValue 时输入框同步', async () => {
    const w = mountFp(FpInput, { props: { modelValue: 'a' } })
    await w.setProps({ modelValue: 'b' })
    await nextTick()
    expect(w.find('input').element.value).toBe('b')
  })

  it('【无障碍】无可见 label：placeholder 回落为 aria-label，显式 aria-label 优先', () => {
    const w = mountFp(FpInput, { props: { modelValue: '' }, attrs: { placeholder: '搜索' } })
    expect(w.find('input').attributes('aria-label')).toBe('搜索')
    const w2 = mountFp(FpInput, {
      props: { modelValue: '' },
      attrs: { placeholder: '搜索', 'aria-label': '搜索容器' },
    })
    expect(w2.find('input').attributes('aria-label')).toBe('搜索容器')
    expect(
      mountFp(FpInput, { props: { modelValue: '' } })
        .find('input')
        .attributes('aria-label'),
    ).toBeUndefined()
  })

  it('【无障碍】有可见 label：由 <label for> 命名，不设 aria-label', () => {
    const w = mountFp(FpInput, {
      props: { modelValue: '', label: '名称' },
      attrs: { placeholder: 'x' },
    })
    expect(w.find('input').attributes('aria-label')).toBeUndefined()
    expect(w.find('label').attributes('for')).toBe(w.find('input').attributes('id'))
  })
})
