import { describe, it, expect, vi, afterEach } from 'vitest'
import { mountFp } from './helpers'
import PageToolbar from '../PageToolbar.vue'

/**
 * T1-1 组件测试：PageToolbar 列表页工具栏。
 *
 * 搜索框经 `modelValue` 受控（父组件持有查询关键字，便于防抖后发起请求），
 * 左右两个插槽分别放筛选器与主操作按钮。
 */
describe('PageToolbar 列表工具栏', () => {
  it('searchable 默认开启：渲染搜索框并带占位文案', () => {
    const w = mountFp(PageToolbar, { props: { modelValue: '' } })
    const input = w.find('input')
    expect(input.exists()).toBe(true)
    expect(input.attributes('placeholder')).toBe('搜索')
  })

  it('searchable=false：不渲染搜索框（如设置页无搜索语义）', () => {
    const w = mountFp(PageToolbar, { props: { searchable: false } })
    expect(w.find('input').exists()).toBe(false)
  })

  it('searchPlaceholder 覆盖默认占位', () => {
    const w = mountFp(PageToolbar, { props: { modelValue: '', searchPlaceholder: '搜索站点域名' } })
    expect(w.find('input').attributes('placeholder')).toBe('搜索站点域名')
  })

  it('输入触发 update:modelValue（受控搜索）', async () => {
    const w = mountFp(PageToolbar, { props: { modelValue: '' } })
    await w.find('input').setValue('nginx')
    expect(w.emitted('update:modelValue')?.at(-1)).toEqual(['nginx'])
  })

  it('left / actions 插槽各就各位', () => {
    const w = mountFp(PageToolbar, {
      props: { modelValue: '' },
      slots: {
        left: '<span class="filter">状态筛选</span>',
        actions: '<button class="create">新建</button>',
      },
    })
    expect(w.find('.toolbar-left .filter').text()).toBe('状态筛选')
    expect(w.find('.toolbar-right .create').text()).toBe('新建')
  })

  it('插槽缺省：布局容器仍渲染（不塌陷）', () => {
    const w = mountFp(PageToolbar, { props: { modelValue: '' } })
    expect(w.find('.page-toolbar').exists()).toBe(true)
    expect(w.find('.toolbar-left').exists()).toBe(true)
    expect(w.find('.toolbar-right').exists()).toBe(true)
  })
})

/**
 * `/` 快捷聚焦：列表页的高频动作是「按 / 直接开始搜」。
 *
 * 契约：焦点不在输入控件内、且无修饰键时，`/` 聚焦搜索框并阻止默认输入；
 * 反之（正在别的输入框里打字、Ctrl+/ 等组合键）一律不劫持——否则用户在
 * 备注框里输不了斜杠。监听挂在 window 上，组件卸载必须解绑，避免切页后
 * 残留的监听把焦点拽到已销毁的节点上。
 */
describe('PageToolbar `/` 快捷聚焦', () => {
  afterEach(() => {
    document.body.innerHTML = ''
    vi.restoreAllMocks()
  })

  const press = (target: EventTarget, init: KeyboardEventInit = {}) => {
    const ev = new KeyboardEvent('keydown', { key: '/', bubbles: true, cancelable: true, ...init })
    const notPrevented = target.dispatchEvent(ev)
    return !notPrevented
  }

  it('焦点空闲时按 `/`：搜索框获得焦点且默认行为被阻止', () => {
    const w = mountFp(PageToolbar, { props: { modelValue: '' }, attachTo: document.body })
    const prevented = press(window)
    expect(prevented).toBe(true)
    expect(document.activeElement).toBe(w.find('input').element)
    w.unmount()
  })

  it('带修饰键（Ctrl+/ 等）不劫持', () => {
    const w = mountFp(PageToolbar, { props: { modelValue: '' }, attachTo: document.body })
    expect(press(window, { ctrlKey: true })).toBe(false)
    expect(document.activeElement).not.toBe(w.find('input').element)
    w.unmount()
  })

  it('焦点在其它输入控件内时不劫持（备注框里要能输斜杠）', () => {
    const w = mountFp(PageToolbar, { props: { modelValue: '' }, attachTo: document.body })
    const textarea = document.createElement('textarea')
    document.body.appendChild(textarea)
    textarea.focus()
    expect(press(textarea)).toBe(false)
    expect(document.activeElement).toBe(textarea)
    w.unmount()
  })

  it('非 `/` 按键忽略', () => {
    const w = mountFp(PageToolbar, { props: { modelValue: '' }, attachTo: document.body })
    expect(press(window, { key: 'k' })).toBe(false)
    expect(document.activeElement).not.toBe(w.find('input').element)
    w.unmount()
  })

  it('searchable=false：不注册全局 keydown 监听', () => {
    const add = vi.spyOn(window, 'addEventListener')
    const w = mountFp(PageToolbar, { props: { searchable: false } })
    expect(add.mock.calls.some(([type]) => type === 'keydown')).toBe(false)
    w.unmount()
  })

  it('卸载后解绑监听：注册与移除的是同一个 keydown 处理器', () => {
    const add = vi.spyOn(window, 'addEventListener')
    const remove = vi.spyOn(window, 'removeEventListener')
    const w = mountFp(PageToolbar, { props: { modelValue: '' }, attachTo: document.body })
    const handler = add.mock.calls.find(([type]) => type === 'keydown')?.[1]
    expect(handler).toBeTypeOf('function')
    w.unmount()
    expect(remove.mock.calls.some(([type, fn]) => type === 'keydown' && fn === handler)).toBe(true)
  })
})
