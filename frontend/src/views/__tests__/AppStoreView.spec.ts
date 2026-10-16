import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { defineComponent, nextTick } from 'vue'
import { createI18n } from 'vue-i18n'
import { createPinia } from 'pinia'
import AppStoreView from '@/views/AppStoreView.vue'

/**
 * T0-4 视图级回归（Doc/23 §3.1 / 问题 P0-6）：应用商店描述随界面语言切换。
 *
 * 历史缺陷：卡片直接渲染 `pkg.short_desc_zh`，en/ja 界面下描述仍是中文。
 * 本 spec 从视图层验证「en-US 显示英文、zh-CN 显示中文、切换语言即时更新」，
 * 是 P0-6 验收标准（en-US 下应用卡片显示英文描述）的机器可读证据。
 *
 * 挂载策略：`shallow` + 仅保留 store 页签槽位——本用例只关心描述渲染，
 * Fp* 组件与网络层一并打桩，避免 OpenVue 弹窗/表格在 jsdom 下的宿主依赖。
 * T2-4 拆分后描述渲染迁到 `AppStoreGrid`，故对该子组件显式解除打桩（其余照旧）。
 */

vi.mock('@/components/ui/FpToast', () => ({
  useFpToast: () => ({
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
    info: vi.fn(),
    show: vi.fn(),
  }),
}))
vi.mock('@/components/ui/FpConfirm', () => ({ useFpConfirm: () => ({ confirmAction: vi.fn() }) }))

vi.mock('@/api/appStore', () => ({
  listPackages: vi.fn().mockResolvedValue({
    data: {
      packages: [
        {
          key: 'gitea',
          name: 'Gitea',
          category: 'tools',
          short_desc_zh: '轻量 Git 服务',
          short_desc_en: 'Lightweight Git hosting',
          tags: [],
          format: 'flame',
          modes: ['container'],
          versions: ['1.0.0'],
          default_version: '1.0.0',
          architectures: [],
        },
      ],
    },
  }),
  listInstalledApps: vi.fn().mockResolvedValue({ data: [] }),
  listWasmBuiltins: vi.fn().mockResolvedValue({ data: [] }),
  listSources: vi.fn().mockResolvedValue({ data: { total: 0, sources: [] } }),
}))

/** 仅渲染 store 页签（其余页签的表格/表单与本用例无关） */
const StoreTabs = defineComponent({
  name: 'FpTabs',
  template: '<div><slot name="store" /></div>',
})

const LayoutStub = defineComponent({ name: 'LayoutContent', template: '<div><slot name="actions" /><slot name="toolbar" /><slot /></div>' })
async function mountStore(locale = 'zh-CN') {
  const i18n = createI18n({
    legacy: false,
    locale,
    missingWarn: false,
    fallbackWarn: false,
    messages: { 'zh-CN': {}, 'en-US': {}, 'ja-JP': {} },
  })
  const w = mount(AppStoreView, {
    shallow: true,
    global: {
      plugins: [i18n, createPinia()],
      stubs: { FpTabs: StoreTabs, AppStoreGrid: false, LayoutContent: LayoutStub },
      directives: { permission: () => {} },
    },
  })
  // 等列表请求 resolve + 渲染刷新
  await new Promise((resolve) => setTimeout(resolve, 30))
  await nextTick()
  return { w, i18n }
}

describe('T0-4 应用商店描述国际化', () => {
  it('zh-CN 显示中文描述', async () => {
    const { w } = await mountStore('zh-CN')
    expect(w.text()).toContain('轻量 Git 服务')
    expect(w.text()).not.toContain('Lightweight Git hosting')
  })

  it('【核心回归】en-US 显示英文描述（历史实现此处为中文）', async () => {
    const { w } = await mountStore('en-US')
    expect(w.text()).toContain('Lightweight Git hosting')
    expect(w.text()).not.toContain('轻量 Git 服务')
  })

  it('ja-JP 回落中文（后端无日语字段，缺失回退 zh）', async () => {
    const { w } = await mountStore('ja-JP')
    expect(w.text()).toContain('轻量 Git 服务')
    expect(w.text()).not.toContain('Lightweight Git hosting')
  })

  it('切换语言后已渲染卡片描述同步更新（无需重挂载）', async () => {
    const { w, i18n } = await mountStore('zh-CN')
    expect(w.text()).toContain('轻量 Git 服务')

    ;(i18n.global.locale as unknown as { value: string }).value = 'en-US'
    await nextTick()
    expect(w.text()).toContain('Lightweight Git hosting')
    expect(w.text()).not.toContain('轻量 Git 服务')
  })
})
