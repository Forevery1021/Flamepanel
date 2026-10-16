import { describe, it, expect, vi } from 'vitest'
import { mountFp } from '@/components/ui/__tests__/helpers'
import AppStoreGrid from '@/components/appStore/AppStoreGrid.vue'

/**
 * T2-4 拆分后的卡片网格组件契约（原为 AppStoreView 内联模板）。
 *
 * 覆盖三件事：① 商店 / WASM 两种形态的差异收口（图标、格式标签、按钮文案）；
 * ② 已安装角标语义；③ 点击卡片上抛 `install` 事件（数据层在 composable）。
 */

vi.mock('@/components/ui/FpToast', () => ({
  useFpToast: () => ({ success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() }),
}))

const messages = {
  appStore: {
    recommended: '推荐安装',
    installed: '已安装',
    install: '安装',
    empty: '暂无应用',
  },
}

const pkg = (key: string, over: Record<string, unknown> = {}) => ({
  key,
  name: key.toUpperCase(),
  category: 'tools',
  short_desc_zh: `${key} 中文描述`,
  short_desc_en: `${key} english`,
  tags: ['tag1'],
  format: 'flame',
  modes: ['container'],
  versions: ['1.0.0'],
  default_version: '1.0.0',
  architectures: [],
  ...over,
})

function mountGrid(props: Record<string, unknown>) {
  // `v-permission` 在单测中无全局注册（生产在 main.ts 注册）；打桩为无操作指令，
  // 与本仓库既有的组件测试一致（本 spec 不验证权限语义，那由 permission.spec 覆盖）
  return mountFp(AppStoreGrid as never, {
    props,
    messages,
    global: { directives: { permission: () => {} } },
  })
}

describe('AppStoreGrid · 商店形态', () => {
  it('渲染卡片名、中英描述与格式标签；空列表落空态', () => {
    const w = mountGrid({ packages: [pkg('gitea')] })
    expect(w.text()).toContain('GITEA')
    expect(w.text()).toContain('gitea 中文描述')
    expect(w.text()).toContain('Flame')
    expect(w.text()).toContain('tag1')
    expect(w.text()).not.toContain('暂无应用')
  })

  it('loading 优先于数据（骨架屏 8 张，避免闪现空态）', () => {
    const w = mountGrid({ packages: [pkg('gitea')], loading: true })
    expect(w.findAll('.app-card-skeleton')).toHaveLength(8)
    expect(w.text()).not.toContain('GITEA')
  })

  it('无数据且非 loading → 空态文案', () => {
    const w = mountGrid({ packages: [] })
    expect(w.text()).toContain('暂无应用')
  })

  it('推荐位单独成区，仅渲染 recommended 项', () => {
    const w = mountGrid({
      packages: [pkg('a'), pkg('b')],
      recommended: [pkg('a', { recommended: true })],
    })
    expect(w.find('.recommend-section').exists()).toBe(true)
    expect(w.find('.recommend-list').text()).toContain('A')
    expect(w.find('.recommend-list').text()).not.toContain('B')
  })

  it('无推荐数据不渲染推荐区（不占位不报错）', () => {
    const w = mountGrid({ packages: [pkg('a')] })
    expect(w.find('.recommend-section').exists()).toBe(false)
  })
})

describe('AppStoreGrid · 已安装角标与事件', () => {
  it('已安装包按钮显示「已安装」并切成功色', () => {
    const w = mountGrid({
      packages: [pkg('gitea'), pkg('redis')],
      installedKeys: new Set(['gitea']),
    })
    const cards = w.findAll('.app-card')
    expect(cards[0].text()).toContain('已安装')
    expect(cards[0].find('button').classes().join(' ')).toContain('success')
    expect(cards[1].text()).toContain('安装')
    expect(cards[1].find('button').classes().join(' ')).not.toContain('success')
  })

  it('点击卡片按钮上抛 install（视图层据此开弹窗）', async () => {
    const w = mountGrid({ packages: [pkg('gitea')] })
    await w.find('.app-card button').trigger('click')
    expect(w.emitted('install')).toBeTruthy()
    expect((w.emitted('install')?.[0]?.[0] as { key: string }).key).toBe('gitea')
  })

  it('点击推荐卡同样上抛 install', async () => {
    const w = mountGrid({ packages: [pkg('a')], recommended: [pkg('a')] })
    await w.find('.recommend-card').trigger('click')
    expect(w.emitted('install')).toBeTruthy()
  })
})

describe('AppStoreGrid · WASM 形态', () => {
  it('wasm 模式：不渲染推荐区与格式标签，图标换 sparkles', () => {
    const w = mountGrid({
      packages: [pkg('t', { category: 'wasm' })],
      wasm: true,
      showRecommend: false,
      recommended: [pkg('t')],
    })
    expect(w.find('.recommend-section').exists()).toBe(false)
    expect(w.find('.app-tags').exists()).toBe(false)
    expect(w.find('.wasm-logo').exists()).toBe(true)
    expect(w.find('.oi-sparkles').exists()).toBe(true)
  })

  it('wasm 模式按钮恒为「安装」主色（无「已安装」语义）', () => {
    const w = mountGrid({
      packages: [pkg('t')],
      wasm: true,
      installedKeys: new Set(['t']),
    })
    expect(w.find('.app-card button').text()).toBe('安装')
    expect(w.find('.app-card button').classes().join(' ')).not.toContain('success')
  })
})
