import { describe, it, expect, vi, afterEach } from 'vitest'
import { mountFp } from './helpers'
import LayoutContent from '../LayoutContent.vue'

/**
 * T1-1 组件测试：LayoutContent 页面骨架。
 *
 * 每个业务页面都套这层壳（标题 + 工具栏 + 内容区），四个插槽的分工必须稳定：
 * `toolbar`/`actions` 落工具栏左右区、`prompt` 落标题下方释义、默认插槽为内容区。
 * 另锁定刷新按钮的装配：仅在 `reload` 时出现，点击发出 `reload` 事件
 * ——「不传 reload 就不会有刷新按钮」是防止误触发的装配约定。
 */
const mountLayout = (props: Record<string, unknown> = {}, slots: Record<string, string> = {}) =>
  mountFp(LayoutContent, {
    props: { title: '网站管理', ...props },
    slots: { default: '<div class="body">主体</div>', ...slots },
  })

describe('LayoutContent 页面骨架', () => {
  it('title 渲染为页面标题', () => {
    const w = mountLayout()
    expect(w.find('.lc-title__text').text()).toBe('网站管理')
  })

  it('默认插槽渲染为内容区', () => {
    const w = mountLayout()
    expect(w.find('.lc-body .body').text()).toBe('主体')
  })

  it('toolbar 插槽落工具栏行，actions 落页头右区', () => {
    const w = mountLayout(
      {},
      { toolbar: '<span class="tb">筛选</span>', actions: '<button class="act">新建</button>' },
    )
    expect(w.find('.lc-toolbar .tb').text()).toBe('筛选')
    expect(w.find('.lc-actions .act').text()).toBe('新建')
  })

  it('prompt 插槽落标题下方（页面释义位）', () => {
    const w = mountLayout({}, { prompt: '<span class="pm">该操作不可撤销</span>' })
    expect(w.find('.lc-prompt .pm').text()).toBe('该操作不可撤销')
  })

  it('prompt 缺省：不渲染释义容器', () => {
    const w = mountLayout()
    expect(w.find('.lc-prompt').exists()).toBe(false)
  })

  it('reload=true：渲染刷新按钮且带无障碍名', () => {
    const w = mountLayout({ reload: true })
    const btn = w.find('.lc-actions .fp-btn')
    expect(btn.exists()).toBe(true)
    expect(btn.attributes('aria-label')).toBe('刷新')
  })

  it('reload 缺省：不渲染刷新按钮（无刷新语义的页面不出现该按钮）', () => {
    const w = mountLayout()
    expect(w.find('.lc-actions .fp-btn').exists()).toBe(false)
  })

  it('点击刷新按钮发出 reload 事件（数据刷新由页面自己实现）', async () => {
    const w = mountLayout({ reload: true })
    await w.find('.lc-actions .fp-btn').trigger('click')
    expect(w.emitted('reload')).toHaveLength(1)
  })

  it('刷新按钮在刷新过程中进入 loading 态（防重复点击）', async () => {
    const w = mountLayout({ reload: true })
    const btn = w.find('.lc-actions .fp-btn')
    await btn.trigger('click')
    // doReload 内部立即置 reloading=true，按钮渲染为加载中
    expect(w.find('.lc-actions .fp-btn').classes().join(' ')).toContain('loading')
  })
})

/**
 * 页头说明 + 页级 KPI 摘要（description / summary）。
 *
 * 摘要 pill 用于列表页首屏「总数 / 运行中 / 异常」一眼扫读：`tone` 只影响
 * 语义色 class，不传即中性；传 `summary` 插槽可整体替换默认 pill。
 */
describe('LayoutContent 说明与摘要', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it('description 渲染在标题下方；缺省不渲染说明段落', () => {
    expect(mountLayout({ description: '管理本机全部站点' }).find('.lc-desc').text()).toBe(
      '管理本机全部站点',
    )
    expect(mountLayout().find('.lc-desc').exists()).toBe(false)
  })

  it('summary 渲染为 KPI pill：值 + 标签，tone 映射语义 class', () => {
    const w = mountLayout({
      summary: [
        { label: '总数', value: 12 },
        { label: '异常', value: 1, tone: 'danger' },
      ],
    })
    const pills = w.findAll('.lc-pill')
    expect(pills).toHaveLength(2)
    expect(pills[0].find('.lc-pill__value').text()).toBe('12')
    expect(pills[0].find('.lc-pill__label').text()).toBe('总数')
    expect(pills[0].classes().some((c) => c.startsWith('is-'))).toBe(false)
    expect(pills[1].classes()).toContain('is-danger')
  })

  it('summary 缺省或为空：不渲染摘要行（避免空白占位）', () => {
    expect(mountLayout().find('.lc-summary').exists()).toBe(false)
    expect(mountLayout({ summary: [] }).find('.lc-summary').exists()).toBe(false)
  })

  it('summary 插槽可整体替换默认 pill', () => {
    const w = mountLayout({}, { summary: '<span class="custom">自定义摘要</span>' })
    expect(w.find('.lc-summary .custom').text()).toBe('自定义摘要')
    expect(w.find('.lc-pill').exists()).toBe(false)
  })

  it('刷新按钮的 loading 态在 500ms 后自动复位（父组件无需回传状态）', async () => {
    vi.useFakeTimers()
    const w = mountLayout({ reload: true })
    await w.find('.lc-actions .fp-btn').trigger('click')
    expect(w.find('.lc-actions .fp-btn').classes().join(' ')).toContain('loading')
    await vi.advanceTimersByTimeAsync(500)
    expect(w.find('.lc-actions .fp-btn').classes().join(' ')).not.toContain('loading')
  })
})
