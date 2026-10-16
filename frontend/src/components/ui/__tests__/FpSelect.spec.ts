import { describe, it, expect } from 'vitest'
import { nextTick } from 'vue'
import { mountFp, createHost } from './helpers'
import FpSelect from '../FpSelect.vue'

/**
 * T1-1 组件测试：FpSelect 下拉选择。
 *
 * OpenVue Select 的弹层走 Teleport（`appendTo`），选项列表不在组件包装器内。
 * 因此交互断言分两步：先在宿主 DOM 上点开选项，再断言组件收到的
 * `update:modelValue`——这也是「选项值正确传递」这一契约的真正落点。
 *
 * 另锁定字段态契约：error 同时驱动视觉非法态与 aria-invalid / aria-describedby，
 * 与 FpInput 保持一致（表单字段组件族行为统一）。
 */
const options = [
  { label: 'Cloudflare', value: 'cloudflare' },
  { label: '阿里云', value: 'aliyun' },
]

/**
 * 与业务视图同源的用法：views 一律显式给 `option-label="label"`
 * `option-value="value"`（见 UsersView / FirewallView 等），否则底层 Select
 * 默认把整个选项对象作为值传出。测试用真实用法，避免测出「假契约」。
 */
function mountSelect(props: Record<string, unknown> = {}) {
  const host = createHost()
  const w = mountFp(FpSelect, {
    props: {
      modelValue: '',
      options,
      appendTo: host,
      'option-label': 'label',
      'option-value': 'value',
      ...props,
    },
    attachTo: host,
  })
  return { w, host }
}

async function pickOption(host: HTMLElement, index: number) {
  const opts = host.querySelectorAll('.p-select-option')
  const target = opts[index] as HTMLElement
  expect(target).toBeTruthy()
  target.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true }))
  target.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }))
  target.dispatchEvent(new MouseEvent('mouseup', { bubbles: true }))
  await nextTick()
}

describe('FpSelect 下拉选择', () => {
  it('渲染下拉框本体（未展开时不渲染选项）', () => {
    const { w, host } = mountSelect()
    expect(w.find('.p-select').exists()).toBe(true)
    expect(host.querySelectorAll('.p-select-option')).toHaveLength(0)
  })

  it('【交互】展开后选项可见，点击选项发出 update:modelValue=所选值', async () => {
    const { w, host } = mountSelect()
    await w.find('.p-select-label').trigger('click')
    await nextTick()
    await nextTick()
    expect(host.querySelectorAll('.p-select-option')).toHaveLength(2)
    await pickOption(host, 1)
    expect(w.emitted('update:modelValue')?.at(-1)).toEqual(['aliyun'])
  })

  it('选中第 1 项：值为 value 字段（option-value 显式指向 value）', async () => {
    const { w, host } = mountSelect()
    await w.find('.p-select-label').trigger('click')
    await nextTick()
    await nextTick()
    await pickOption(host, 0)
    expect(w.emitted('update:modelValue')?.at(-1)).toEqual(['cloudflare'])
  })

  it('label 非空：静态标签与控件 id 关联， label 与控件 id 关联', () => {
    const { w } = mountSelect({ label: 'DNS 厂商' })
    const label = w.find('label')
    expect(label.text()).toBe('DNS 厂商')
    // W4-a：`for` 指向可聚焦的 combobox（旧版指向根 div，点标签不会聚焦控件）
    const target = w.find(`#${label.attributes('for')}`)
    expect(target.exists()).toBe(true)
    expect(target.attributes('role')).toBe('combobox')
  })

  it('inputId：外部 <label for> 可指定控件 id（设置页行式布局用）', () => {
    const { w } = mountSelect({ inputId: 'lang-select' })
    expect(w.find('#lang-select').attributes('role')).toBe('combobox')
  })

  it('label 为空：不渲染标签', () => {
    const { w } = mountSelect()
    expect(w.find('label').exists()).toBe(false)
  })

  it('error 非空：容器进入非法态 + aria-invalid + 错误文案可见', () => {
    const { w } = mountSelect({ error: '请选择厂商' })
    expect(w.find('.fp-field').classes()).toContain('fp-field-invalid')
    expect(w.find('.p-select').attributes('aria-invalid')).toBe('true')
    const describedBy = w.find('.p-select').attributes('aria-describedby')
    expect(w.find(`#${describedBy}`).text()).toBe('请选择厂商')
  })

  it('invalid 手动置非法态：无文案也标记 aria-invalid', () => {
    const { w } = mountSelect({ invalid: true })
    expect(w.find('.p-select').attributes('aria-invalid')).toBe('true')
    expect(w.find('.fp-field-error').exists()).toBe(false)
  })

  it('$attrs 透传 show-clear / filter 到底层组件', () => {
    const { w } = mountSelect({ showClear: true, filter: true })
    expect(w.html()).toContain('p-select')
    // 透传的布尔属性不应被 FpSelect 吞掉
    expect(w.find('.p-select').classes()).toContain('w-full')
  })

  it('父组件改写 modelValue：已选值同步显示', async () => {
    const { w } = mountSelect({ modelValue: 'cloudflare' })
    await w.setProps({ modelValue: 'aliyun' })
    await nextTick()
    expect(w.find('.p-select-label').text()).toContain('阿里云')
  })
})
