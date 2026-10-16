import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { defineComponent, h, nextTick, ref } from 'vue'
import FpErrorBoundary from '../FpErrorBoundary.vue'
import { flushErrors } from '@/utils/monitor'

/**
 * R-2 回归：错误边界必须（a）接管子树渲染错误、（b）正常子树不受影响、
 * （c）崩溃时文案可见（不透出裸 key / 空白）、（d）可重试恢复、（e）resetKey 复位。
 *
 * 说明：测试组件统一经 `makeComp` 工厂构造（单处 defineComponent 调用），
 * 避免 spec 文件堆叠组件定义触发 vue/one-component-per-file。
 */

/** 测试组件工厂：`setup` 返回 render 函数即可，抛错即模拟崩溃 */
const makeComp = (name: string, setup: () => unknown) =>
  defineComponent({ name, setup: setup as () => never })

/** 首次渲染即抛错（模拟页面 setup 崩溃） */
const boomSetup = () =>
  makeComp('BoomSetupPage', () => {
    throw new Error('page-boom')
  })

/** 渲染期抛错（setup 正常、渲染函数炸） */
const boomRender = () =>
  makeComp('BoomRenderPage', () => () => {
    throw new Error('render-boom')
  })

const okComp = () => makeComp('OkPage', () => () => h('div', { class: 'ok-content' }, '正常内容'))

/** 首次崩溃、之后正常（验证「重试可恢复」） */
const flakyComp = (state: { shouldThrow: boolean }, cls: string, text: string) =>
  makeComp('FlakyPage', () => {
    if (state.shouldThrow) throw new Error('flaky-boom')
    return () => h('div', { class: cls }, text)
  })

/** 挂载错误边界并注入子树（slots 在边界组件内，错误沿父链被捕获） */
const mountBoundary = (child: unknown, props: Record<string, unknown> = {}) =>
  mount(FpErrorBoundary, { props, slots: { default: () => h(child as never) } })

describe('FpErrorBoundary 错误边界（R-2）', () => {
  beforeEach(() => {
    flushErrors()
    vi.spyOn(console, 'error').mockImplementation(() => {})
    vi.spyOn(console, 'warn').mockImplementation(() => {})
  })
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('正常子树原样渲染，不出现错误面板', async () => {
    const w = mountBoundary(okComp())
    await nextTick()
    expect(w.find('.ok-content').exists()).toBe(true)
    expect(w.find('.fp-eb').exists()).toBe(false)
  })

  it('setup 抛错：接管为错误面板且不透出子组件内容', async () => {
    const w = mountBoundary(boomSetup())
    await nextTick()
    await nextTick()
    expect(w.find('.fp-eb').exists()).toBe(true)
    expect(w.text()).not.toContain('never-rendered')
  })

  it('错误面板文案可见（非空白、非裸 key）', async () => {
    const w = mountBoundary(boomSetup())
    await nextTick()
    await nextTick()
    const text = w.find('.fp-eb').text()
    // 无 i18n 语言包时走硬编码兜底：文案非空且不是 common.* 裸 key
    expect(text.length).toBeGreaterThan(0)
    expect(text).not.toContain('common.')
    // 「界面渲染异常」/「UI rendering error」/「画面の描画エラー」三选一
    expect(text).toMatch(/界面渲染异常|UI rendering error|画面の描画エラー/)
  })

  it('错误详情透出（排障可见原始 message）', async () => {
    const w = mountBoundary(boomRender())
    await nextTick()
    await nextTick()
    expect(w.find('.fp-eb__desc').text()).toBe('render-boom')
  })

  it('点击重试可恢复：面板消失并重新渲染子树', async () => {
    const state = { shouldThrow: true }
    const w = mountBoundary(flakyComp(state, 'recovered', '已恢复'))
    await nextTick()
    await nextTick()
    expect(w.find('.fp-eb').exists()).toBe(true)

    state.shouldThrow = false
    await w.find('.fp-eb__btn').trigger('click')
    await nextTick()
    await nextTick()
    expect(w.find('.fp-eb').exists()).toBe(false)
    expect(w.find('.recovered').exists()).toBe(true)
  })

  it('resetKey 变化清除错误态（切页不残留错误面板）', async () => {
    // 仅首次挂载抛错：复位后子树应恢复正常渲染，而非停留在旧错误面板
    const state = { shouldThrow: true }
    const child = makeComp('OnceFlakyPage', () => {
      if (state.shouldThrow) {
        state.shouldThrow = false
        throw new Error('once-boom')
      }
      return () => h('div', { class: 'ok-after-reset' }, '复位后内容')
    })
    const key = ref('/a')
    const w = mount(FpErrorBoundary, {
      props: { resetKey: key.value },
      slots: { default: () => h(child) },
    })
    await nextTick()
    await nextTick()
    expect(w.find('.fp-eb').exists()).toBe(true)

    await w.setProps({ resetKey: '/b' })
    await nextTick()
    await nextTick()
    expect(w.find('.fp-eb').exists()).toBe(false)
    expect(w.find('.ok-after-reset').exists()).toBe(true)
  })

  it('report=false 时不进行全局上报', async () => {
    const errSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    const w = mountBoundary(boomSetup(), { report: false })
    await nextTick()
    await nextTick()
    expect(w.find('.fp-eb').exists()).toBe(true)
    expect(errSpy).not.toHaveBeenCalled()
  })
})
