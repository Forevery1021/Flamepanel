import { describe, it, expect, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import CommandPalette from '@/components/layout/CommandPalette.vue'
import { createTestI18n } from '@/components/ui/__tests__/helpers'
import { menuRoutes } from '@/router'
import zhCN from '@/locales/zh-CN'

/**
 * S-15 命令面板：空查询分「最近访问 / 全部命令」两组，键盘游标跨组连续；
 * 有查询时只列过滤结果（不重复列最近访问）；最近访问在每次打开时重读。
 */

function mountPalette() {
  const router = createRouter({
    history: createMemoryHistory(),
    // 与真实路由表同构：菜单路由挂在根布局下（子路由 path 为相对路径）
    routes: [{ path: '/', component: { template: '<router-view />' }, children: menuRoutes }],
  })
  const wrapper = mount(CommandPalette, {
    props: { modelValue: false },
    global: {
      plugins: [router, createTestI18n(zhCN as unknown as Record<string, unknown>)],
      // Dialog 打桩为直接渲染默认插槽（本用例只关心列表内容与键盘行为）
      stubs: { Dialog: { template: '<div class="dialog-stub"><slot /></div>' } },
    },
  })
  return { wrapper, router }
}

async function open(wrapper: ReturnType<typeof mountPalette>['wrapper']) {
  await wrapper.setProps({ modelValue: true })
  await nextTick()
  await nextTick()
}

describe('CommandPalette', () => {
  beforeEach(() => {
    localStorage.clear()
    localStorage.setItem('username', 'admin')
    localStorage.setItem('role', 'admin')
    localStorage.setItem('token', 'x')
    setActivePinia(createPinia())
  })

  it('无最近访问时只有一组全部命令，含「切换材质」动作', async () => {
    const { wrapper } = mountPalette()
    await open(wrapper)
    expect(wrapper.findAll('.palette-group')).toHaveLength(1)
    expect(wrapper.find('.palette-group__label').exists()).toBe(false)
    expect(wrapper.text()).toContain('切换材质')
    expect(wrapper.text()).not.toContain('玻璃材质')
  })

  it('打开时读取最近访问并分组展示，游标跨组连续编号', async () => {
    localStorage.setItem(
      'flamepanel.palette.recent.admin',
      JSON.stringify(['Firewall', 'NoSuchRoute', 'Memos']),
    )
    const { wrapper } = mountPalette()
    await open(wrapper)
    const labels = wrapper.findAll('.palette-group__label').map((l) => l.text())
    expect(labels).toEqual(['最近访问', '全部命令'])
    const recentGroup = wrapper.findAll('.palette-group')[0]
    // 不存在的路由名被丢弃，顺序保持最近优先
    expect(recentGroup.findAll('.palette-item')).toHaveLength(2)
    expect(recentGroup.findAll('.palette-item__label').map((l) => l.text())).toEqual([
      '防火墙',
      '备忘录',
    ])
    // 第二组首项的扁平下标紧接最近访问之后
    const allGroup = wrapper.findAll('.palette-group')[1]
    expect(allGroup.find('.palette-item').attributes('id')).toBe('palette-option-2')

    const input = wrapper.find('input')
    await input.trigger('keydown', { key: 'ArrowDown' })
    await input.trigger('keydown', { key: 'ArrowDown' })
    expect(wrapper.find('.palette-item.is-selected').attributes('id')).toBe('palette-option-2')
  })

  it('有查询时只显示过滤结果，不再重复列出最近访问；回车执行导航', async () => {
    localStorage.setItem('flamepanel.palette.recent.admin', JSON.stringify(['Firewall']))
    const { wrapper, router } = mountPalette()
    await open(wrapper)
    const input = wrapper.find('input')
    await input.setValue('防火墙')
    expect(wrapper.find('.palette-group__label').exists()).toBe(false)
    const items = wrapper.findAll('.palette-item__label').map((l) => l.text())
    expect(items.filter((l) => l === '防火墙')).toHaveLength(1)
    await input.trigger('keydown', { key: 'Enter' })
    await router.isReady()
    await new Promise((r) => setTimeout(r, 0))
    expect(router.currentRoute.value.name).toBe('Firewall')
    expect(wrapper.emitted('update:modelValue')?.at(-1)).toEqual([false])
  })
})
