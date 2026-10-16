import { describe, it, expect, afterEach } from 'vitest'
import { defineComponent, nextTick, ref, type Ref } from 'vue'
import { mount } from '@vue/test-utils'
import { layoutBand, useResponsiveLayout } from '@/composables/useResponsiveLayout'

/**
 * 平板断点审计：768 宽 iPad 竖屏不再被当手机；只在跨越断点时调整折叠，
 * 同档位内的 resize（地址栏 / 软键盘只改高度）不覆盖用户的手动选择。
 */

let width = 1440
let listeners: (() => void)[] = []
const original = window.matchMedia

function installFakeMatchMedia() {
  window.matchMedia = ((query: string) => {
    const max = Number(/max-width:\s*(\d+)px/.exec(query)?.[1] ?? Infinity)
    return {
      media: query,
      get matches() {
        return width <= max
      },
      addEventListener: (_: string, cb: () => void) => listeners.push(cb),
      removeEventListener: (_: string, cb: () => void) => {
        const i = listeners.indexOf(cb)
        if (i >= 0) listeners.splice(i, 1)
      },
    } as unknown as MediaQueryList
  }) as typeof window.matchMedia
}

async function resize(w: number) {
  const crossed = layoutBand(w) !== layoutBand(width)
  width = w
  // 真实浏览器只在查询结果翻转时派发 change
  if (crossed) for (const cb of new Set(listeners)) cb()
  await nextTick()
}

function mountAt(w: number, initialCollapsed = false) {
  width = w
  installFakeMatchMedia()
  const collapsed = ref(initialCollapsed)
  let api!: ReturnType<typeof useResponsiveLayout>
  const Host = defineComponent({
    setup() {
      api = useResponsiveLayout(collapsed as Ref<boolean>)
      return () => null
    },
  })
  const wrapper = mount(Host)
  return { wrapper, collapsed, api }
}

afterEach(() => {
  listeners = []
  window.matchMedia = original
})

describe('layoutBand', () => {
  it.each([
    [375, 'mobile'],
    [767, 'mobile'],
    [768, 'tablet'],
    [1024, 'tablet'],
    [1199, 'tablet'],
    [1200, 'desktop'],
  ])('%ipx → %s', (w, band) => {
    expect(layoutBand(w)).toBe(band)
  })
})

describe('useResponsiveLayout', () => {
  it('iPad 竖屏 768：常驻侧栏且首屏折叠（非抽屉）', () => {
    const { api, collapsed } = mountAt(768)
    expect(api.isMobile.value).toBe(false)
    expect(api.band.value).toBe('tablet')
    expect(collapsed.value).toBe(true)
  })

  it('桌面首屏保留持久化偏好（折叠的仍折叠）', () => {
    expect(mountAt(1440, true).collapsed.value).toBe(true)
    expect(mountAt(1440, false).collapsed.value).toBe(false)
  })

  it('平板内手动展开后，同档 resize 不再强制折叠', async () => {
    const { collapsed } = mountAt(1024)
    collapsed.value = true
    collapsed.value = false // 用户手动展开
    await resize(1000)
    expect(collapsed.value).toBe(false)
  })

  it('跨档：桌面→平板折叠，平板→桌面展开；桌面内手动折叠不被 resize 撤销', async () => {
    const { collapsed } = mountAt(1440)
    await resize(1100)
    expect(collapsed.value).toBe(true)
    await resize(1300)
    expect(collapsed.value).toBe(false)
    collapsed.value = true
    await resize(1500)
    expect(collapsed.value).toBe(true)
  })

  it('进入手机档切到抽屉并关闭；回到平板恢复常驻折叠侧栏', async () => {
    const { api, collapsed } = mountAt(1024)
    api.mobileOpen.value = true
    await resize(390)
    expect(api.isMobile.value).toBe(true)
    expect(api.mobileOpen.value).toBe(false)
    collapsed.value = false
    await resize(820)
    expect(api.isMobile.value).toBe(false)
    expect(collapsed.value).toBe(true)
  })

  it('卸载时移除监听', () => {
    const { wrapper } = mountAt(1024)
    expect(listeners).toHaveLength(2)
    wrapper.unmount()
    expect(listeners).toHaveLength(0)
  })
})
