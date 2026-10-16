import { ref, onMounted, onUnmounted, type Ref } from 'vue'

/**
 * 布局断点（平板审计 batch-10）。
 *
 * - mobile  ≤ 767px：侧栏收进抽屉；
 * - tablet  768–1199px：侧栏常驻但默认折叠为图标栏（iPad 竖屏 768 / 横屏 1024 均落在此档）；
 * - desktop ≥ 1200px：侧栏默认展开。
 *
 * 旧实现的问题：①`max-width: 768px` 把 768 宽的 iPad 竖屏当成手机；②每次 `resize`
 * （包括 iPad 地址栏伸缩、软键盘这类只改高度的事件）都强制按宽度重置折叠，用户手动展开 /
 * 折叠的选择被立即覆盖。现在改为监听 matchMedia 的 `change`——只在**跨越断点**时调整，
 * 同一档位内尊重用户选择。
 */
export const MOBILE_MAX = 767
export const TABLET_MAX = 1199

export type LayoutBand = 'mobile' | 'tablet' | 'desktop'

export function layoutBand(width: number): LayoutBand {
  if (width <= MOBILE_MAX) return 'mobile'
  if (width <= TABLET_MAX) return 'tablet'
  return 'desktop'
}

export function useResponsiveLayout(collapsed: Ref<boolean>) {
  const isMobile = ref(false)
  const mobileOpen = ref(false)
  const band = ref<LayoutBand>('desktop')
  let mobileMql: MediaQueryList | null = null
  let compactMql: MediaQueryList | null = null

  function apply(next: LayoutBand, initial: boolean) {
    const prev = band.value
    band.value = next
    isMobile.value = next === 'mobile'
    if (next === 'mobile') {
      mobileOpen.value = false
      return
    }
    // 首屏：平板强制折叠（桌面保留持久化 / 后端同步的偏好）；
    // 跨档：进入平板折叠，进入桌面展开；从手机回到平板 / 桌面同理。
    if (initial) {
      if (next === 'tablet') collapsed.value = true
    } else if (prev !== next) {
      collapsed.value = next === 'tablet'
    }
  }

  function current(): LayoutBand {
    if (mobileMql?.matches) return 'mobile'
    if (compactMql?.matches) return 'tablet'
    return 'desktop'
  }
  const onChange = () => apply(current(), false)

  onMounted(() => {
    mobileMql = window.matchMedia(`(max-width: ${MOBILE_MAX}px)`)
    compactMql = window.matchMedia(`(max-width: ${TABLET_MAX}px)`)
    apply(current(), true)
    mobileMql.addEventListener('change', onChange)
    compactMql.addEventListener('change', onChange)
  })
  onUnmounted(() => {
    mobileMql?.removeEventListener('change', onChange)
    compactMql?.removeEventListener('change', onChange)
  })

  return { isMobile, mobileOpen, band }
}
