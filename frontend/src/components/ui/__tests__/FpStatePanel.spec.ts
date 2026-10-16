import { describe, it, expect, vi, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import FpStatePanel from '../FpStatePanel.vue'

/**
 * R-2 回归：错误态判定与文案自证。
 *
 * 修复前的两个缺陷：
 * 1. 只认 `error` prop——传 `title`/`description` 不传 `error` 时三态全不命中，
 *    组件渲染为空（App 级致命错误兜底曾因此显示空白）；
 * 2. 文案直接 `t('common.failed')`——语言包未加载时上屏裸 key，等于是「兜底失效」。
 */

/** 未装载任何语言包的 i18n：模拟启动期语言包尚未懒加载完成 */
const emptyI18n = createI18n({ legacy: false, locale: 'zh-CN', messages: {} })
const g = { global: { plugins: [emptyI18n] } }

describe('FpStatePanel 错误态（R-2）', () => {
  it('【关键回归】只传 title+description（不传 error）也进入错误态', () => {
    const w = mount(FpStatePanel, {
      props: { title: '致命错误', description: 'boom', retryable: true },
      ...g,
    })
    expect(w.find('.fp-state__error').exists()).toBe(true)
    expect(w.text()).toContain('致命错误')
    expect(w.text()).toContain('boom')
    // 历史缺陷：此处曾整块空白
    expect(w.text().trim().length).toBeGreaterThan(0)
  })

  it('error 非空进入错误态，description 缺省取 error', () => {
    const w = mount(FpStatePanel, { props: { error: 'read failed' }, ...g })
    expect(w.find('.fp-state__error').exists()).toBe(true)
    expect(w.find('.fp-state__desc').text()).toBe('read failed')
  })

  it('语言包未装载时不透出裸 key（走硬编码兜底）', () => {
    const w = mount(FpStatePanel, { props: { error: 'boom', retryable: true }, ...g })
    const text = w.text()
    expect(text).not.toContain('common.')
    // 重试按钮文案为三语言之一
    expect(text).toMatch(/重试|Retry|再試行/)
    expect(text).toMatch(/操作失败|Operation failed|操作失敗/)
  })

  it('有语言包时优先走 i18n 文案', () => {
    const i18n = createI18n({
      legacy: false,
      locale: 'zh-CN',
      messages: {
        'zh-CN': { common: { failed: '自定义失败', retry: '自定义重试', noData: '空' } },
      },
    })
    const w = mount(FpStatePanel, {
      props: { error: 'boom', retryable: true },
      global: { plugins: [i18n] },
    })
    expect(w.text()).toContain('自定义失败')
    expect(w.text()).toContain('自定义重试')
  })

  it('空态文案同样不透出裸 key', () => {
    const w = mount(FpStatePanel, { props: { empty: true }, ...g })
    expect(w.find('.fp-state__empty').exists()).toBe(true)
    expect(w.text()).not.toContain('common.')
  })

  it('loading 优先于 error / empty', () => {
    const w = mount(FpStatePanel, {
      props: { loading: true, error: 'boom', empty: true },
      ...g,
    })
    expect(w.find('.fp-state__loading').exists()).toBe(true)
    expect(w.find('.fp-state__error').exists()).toBe(false)
  })

  it('无任何状态时渲染默认插槽内容', () => {
    const w = mount(FpStatePanel, {
      slots: { default: '<div class="body">正文</div>' },
      ...g,
    })
    expect(w.find('.body').exists()).toBe(true)
  })
})

/**
 * 三态之外的装配细节：重试按钮、空态动作位、骨架/转圈两种加载态、hideEmpty。
 *
 * 重试按钮点击后本地进入 loading 并延时复位——父组件只需监听 `retry` 重新拉数，
 * 无需回传状态；600ms 内的重复点击被按钮 loading 态吞掉。
 */
describe('FpStatePanel 装配细节', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it('点击重试：发出 retry 事件并进入 loading，600ms 后复位', async () => {
    vi.useFakeTimers()
    const w = mount(FpStatePanel, {
      props: { error: 'boom', retryable: true, retryText: '再来一次' },
      ...g,
    })
    const btn = w.find('.fp-state__error button')
    expect(btn.text()).toBe('再来一次')
    await btn.trigger('click')
    expect(w.emitted('retry')).toHaveLength(1)
    expect(w.find('.fp-state__error button').classes().join(' ')).toContain('loading')
    await vi.advanceTimersByTimeAsync(600)
    expect(w.find('.fp-state__error button').classes().join(' ')).not.toContain('loading')
  })

  it('retryable 缺省：错误态不渲染重试按钮', () => {
    const w = mount(FpStatePanel, { props: { error: 'boom' }, ...g })
    expect(w.find('.fp-state__error button').exists()).toBe(false)
  })

  it('空态：emptyTitle/emptyDesc 透传，emptyAction 插槽落动作位', () => {
    const w = mount(FpStatePanel, {
      props: { empty: true, emptyTitle: '还没有站点', emptyDesc: '创建第一个站点开始使用' },
      slots: { emptyAction: '<button class="create">新建站点</button>' },
      ...g,
    })
    expect(w.find('.fp-empty__title').text()).toBe('还没有站点')
    expect(w.find('.fp-empty__desc').text()).toBe('创建第一个站点开始使用')
    expect(w.find('.fp-empty__action .create').text()).toBe('新建站点')
  })

  it('hideEmpty：数据为空时不显示空态，直接渲染默认插槽（如筛选后无结果的表格）', () => {
    const w = mount(FpStatePanel, {
      props: { empty: true, hideEmpty: true },
      slots: { default: '<div class="body">表格</div>' },
      ...g,
    })
    expect(w.find('.fp-state__empty').exists()).toBe(false)
    expect(w.find('.body').exists()).toBe(true)
  })

  it('加载态默认为表格骨架，行数由 skeletonRows 控制', () => {
    const w = mount(FpStatePanel, { props: { loading: true, skeletonRows: 3 }, ...g })
    expect(w.findAll('.fp-state__skeleton-row')).toHaveLength(3)
    expect(w.find('.fp-state__spinner').exists()).toBe(false)
  })

  it('skeleton=false：转圈 + loadingText（非列表场景）', () => {
    const w = mount(FpStatePanel, {
      props: { loading: true, skeleton: false, loadingText: '正在连接…' },
      ...g,
    })
    expect(w.find('.fp-state__spinner').exists()).toBe(true)
    expect(w.find('.fp-state__text').text()).toBe('正在连接…')
    expect(w.find('.fp-state__skeleton').exists()).toBe(false)
  })

  it('loading / error / empty 插槽可整体替换默认呈现', () => {
    const loading = mount(FpStatePanel, {
      props: { loading: true },
      slots: { loading: '<i class="my-loading" />' },
      ...g,
    })
    expect(loading.find('.my-loading').exists()).toBe(true)
    expect(loading.find('.fp-state__skeleton').exists()).toBe(false)

    const error = mount(FpStatePanel, {
      props: { error: 'boom' },
      slots: { error: '<i class="my-error" />' },
      ...g,
    })
    expect(error.find('.my-error').exists()).toBe(true)
    expect(error.find('.fp-state__title').exists()).toBe(false)

    const empty = mount(FpStatePanel, {
      props: { empty: true },
      slots: { empty: '<i class="my-empty" />' },
      ...g,
    })
    expect(empty.find('.my-empty').exists()).toBe(true)
    expect(empty.find('.fp-empty').exists()).toBe(false)
  })
})
