import { describe, it, expect } from 'vitest'
import { mountFp } from './helpers'
import FpTabs from '../FpTabs.vue'
import type { FpTabItem } from '../FpTabs.vue'

/**
 * T1-1 组件测试：FpTabs 声明式页签。
 *
 * `items` 是声明式契约（文案/图标来自 i18n 与 openicons），命名插槽按
 * `value` 自动落到对应面板。这里同时锁定一个易错点：点击页签发出的是
 * `update:modelValue`，且值必须与 `value` 字面一致（数字被 String() 归一）。
 */
const items: FpTabItem[] = [
  { value: 'containers', label: '容器' },
  { value: 'images', label: '镜像', icon: 'oi oi-box' },
]

const mountTabs = (modelValue = 'containers') =>
  mountFp(FpTabs, {
    props: { modelValue, items },
    slots: {
      containers: '<div class="pane-c">容器面板</div>',
      images: '<div class="pane-i">镜像面板</div>',
    },
  })

describe('FpTabs 页签', () => {
  it('items 声明式渲染页签按钮（数量与顺序一致）', () => {
    const w = mountTabs()
    const tabs = w.findAll('.p-tab')
    expect(tabs).toHaveLength(2)
    expect(tabs[0]!.text()).toContain('容器')
    expect(tabs[1]!.text()).toContain('镜像')
  })

  it('item.icon 渲染为 openicons 图标元素', () => {
    const w = mountTabs()
    const icon = w.findAll('.p-tab')[1]!.find('.fp-tabs__icon')
    expect(icon.exists()).toBe(true)
    expect(icon.classes()).toContain('oi-box')
  })

  it('当前项对应的命名插槽渲染为激活面板', () => {
    const w = mountTabs('containers')
    expect(w.find('.p-tabpanel-active .pane-c').text()).toBe('容器面板')
    // 非激活面板存在但隐藏（保留滚动位置与组件状态）
    expect(w.find('.pane-i').exists()).toBe(true)
  })

  it('点击另一页签：发出 update:modelValue 且面板随之切换', async () => {
    const w = mountTabs('containers')
    await w.findAll('.p-tab')[1]!.trigger('click')
    expect(w.emitted('update:modelValue')?.at(-1)).toEqual(['images'])
    await w.setProps({ modelValue: 'images' })
    expect(w.find('.p-tabpanel-active .pane-i').text()).toBe('镜像面板')
  })

  it('激活项带 data-p-active 标记（供样式与断言定位）', () => {
    const w = mountTabs('images')
    const active = w.findAll('.p-tab').find((t) => t.attributes('data-p-active') === 'true')
    expect(active?.text()).toContain('镜像')
  })

  it('items 为空：不渲染任何页签按钮（不崩）', () => {
    const w = mountFp(FpTabs, { props: { modelValue: 'x', items: [] } })
    expect(w.findAll('.p-tab')).toHaveLength(0)
    expect(w.find('.p-tabs').exists()).toBe(true)
  })

  it('item.badge 渲染角标，0 不显示', () => {
    const w = mountFp(FpTabs, {
      props: {
        modelValue: 'a',
        items: [
          { value: 'a', label: 'A', badge: 3 },
          { value: 'b', label: 'B', badge: 0 },
        ],
      },
    })
    const tabs = w.findAll('.p-tab')
    expect(tabs[0]!.find('.fp-tabs__badge').text()).toBe('3')
    expect(tabs[1]!.find('.fp-tabs__badge').exists()).toBe(false)
  })
})
