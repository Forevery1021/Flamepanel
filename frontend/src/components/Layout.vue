<template>
  <div class="layout" :class="{ 'is-collapsed': collapsed, 'is-mobile': isMobile }">
    <div v-if="appBg" class="layout-bg" :style="{ backgroundImage: appBg }" aria-hidden="true" />
    <AppSidebar v-if="!isMobile" :collapsed="collapsed" class="layout-sidebar" />
    <Drawer
      v-else
      v-model:visible="mobileOpen"
      position="left"
      :modal="true"
      :show-close-icon="false"
      class="mobile-drawer"
    >
      <AppSidebar :collapsed="false" />
    </Drawer>

    <div class="layout-right">
      <AppHeader
        :collapsed="collapsed"
        :is-mobile="isMobile"
        @toggle-collapse="toggleCollapse"
        @open-mobile="mobileOpen = true"
      />
      <main class="main">
        <!-- R-2：页面级错误边界——单页崩溃只替换内容区，侧栏/顶栏保留，用户可切菜单自救 -->
        <FpErrorBoundary :reset-key="route.fullPath">
          <router-view v-slot="{ Component }">
            <transition name="page" mode="out-in">
              <keep-alive :include="keepAliveNames">
                <component :is="Component" />
              </keep-alive>
            </transition>
          </router-view>
        </FpErrorBoundary>
        <span ref="mainHeadingRef" tabindex="-1" class="sr-only" />
      </main>
      <AppFooter />
    </div>

    <!-- UX-2：首次进入面板的操作向导（可跳过；完成/跳过后按用户名持久化，不再打扰） -->
    <OnboardingTour />
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed, nextTick, defineAsyncComponent } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useStorage } from '@vueuse/core'
import Drawer from 'openvue/drawer'
import FpErrorBoundary from '@/components/ui/FpErrorBoundary.vue'
import AppSidebar from './layout/AppSidebar.vue'
import AppHeader from './layout/AppHeader.vue'
import AppFooter from './layout/AppFooter.vue'
import { useThemeStore } from '@/stores/theme'
import { useAppearanceStore } from '@/stores/appearance'
import { STORAGE_KEYS, rawBooleanSerializer } from '@/utils/storage'
import { useResponsiveLayout } from '@/composables/useResponsiveLayout'

/**
 * 主布局（UX-1 重构）。
 *
 * 与上一版的差异：
 * - **移除多页签栏**（`AppTabs` + `stores/tabs`）：顶栏之下不再有 38px 的页签条，
 *   首屏内容区高度 +38px（1080p 下约 +4.5%），页面切换由侧栏与 ⌘K 承担；
 * - `keep-alive` 的 include 列表不再由「已打开页签」推导，而是**静态**来自路由表
 *   `meta.keepAlive === true` 的视图（`router/keepAliveComponentNames`）——语义更直接：
 *   声明了 keepAlive 的页面切走再切回保留状态，其余（Terminal 等有状态视图）每次重建；
 * - 挂载 `OnboardingTour`（懒加载：只有首次进入的用户才会真正下载该 chunk）。
 */
const OnboardingTour = defineAsyncComponent(
  () => import('@/components/onboarding/OnboardingTour.vue'),
)

const route = useRoute()
const { t } = useI18n()
const themeStore = useThemeStore()
const appearance = useAppearanceStore()
const router = useRouter()
/**
 * keep-alive 白名单：从**注入的** router 实例读取（而非 import 应用单例），
 * 便于单测注入内存路由；命名规则与 `router/keepAliveComponentNames` 一致。
 */
const keepAliveNames = computed(() =>
  router
    .getRoutes()
    .filter((r) => r.meta.keepAlive === true)
    .map((r) => r.meta.viewName ?? `${String(r.name)}View`),
)
const appBg = computed(() =>
  themeStore.custom.appBackground?.startsWith('data:') ||
  themeStore.custom.appBackground?.startsWith('http')
    ? `url("${themeStore.custom.appBackground}")`
    : '',
)
// P6：改用 @vueuse/core useStorage 统一持久化侧边栏折叠状态
const collapsed = useStorage<boolean>(STORAGE_KEYS.collapse, false, undefined, {
  serializer: rawBooleanSerializer,
  writeDefaults: false,
})
collapsed.value = collapsed.value || appearance.state.menuCollapsed
const { isMobile, mobileOpen } = useResponsiveLayout(collapsed)

// 后端设置驱动折叠状态（多端同步）
watch(
  () => appearance.state.menuCollapsed,
  (v) => {
    if (isMobile.value) return
    collapsed.value = v
  },
)

function toggleCollapse() {
  const next = !collapsed.value
  collapsed.value = next
  appearance.update({ menuCollapsed: next })
}

// 路由切换后把焦点移到主内容区（屏幕阅读器可感知页面变化，键盘用户可继续操作）
const mainHeadingRef = ref<HTMLElement>()
watch(
  () => route.fullPath,
  async () => {
    if (route.path.startsWith('/login')) return
    await nextTick()
    // 仅键盘/辅助技术场景才移动焦点，避免干扰鼠标用户
    const el = mainHeadingRef.value
    if (el && route.meta?.title) {
      el.textContent = t(route.meta.title)
      el.focus({ preventScroll: true })
    }
    if (isMobile.value) mobileOpen.value = false
  },
)
</script>

<style scoped>
.layout {
  position: relative;
  display: flex;
  /* 环境光斑垫底（solid 材质下 --fp-ambient = none，退化为纯色） */
  background: var(--fp-ambient), var(--fp-bg-app);
  background-attachment: fixed;
  height: 100vh;
  height: 100dvh;
  overflow: hidden;
}
.layout-bg {
  position: absolute;
  inset: 0;
  background-size: cover;
  background-position: center;
  background-repeat: no-repeat;
  background-attachment: fixed;
  z-index: 0;
  pointer-events: none;
}
.layout-right {
  position: relative;
  z-index: 1;
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.main {
  flex: 1;
  padding: var(--fp-space-5);
  overflow: auto;
  background: transparent;
  /* 滚动条常驻占位，避免内容高度变化时横向抖动 */
  scrollbar-gutter: stable;
}

/* 页面切换过渡 */
.page-enter-active,
.page-leave-active {
  transition:
    opacity var(--fp-transition-base),
    transform var(--fp-transition-base);
}
.page-enter-from {
  opacity: 0;
  transform: translateY(6px);
}
.page-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

.mobile-drawer :deep(.p-drawer-content) {
  padding: 0;
}
.mobile-drawer :deep(.p-drawer) {
  width: 240px;
}

@media (max-width: 767px) {
  .main {
    padding: var(--fp-space-3);
  }
}

/* 平板宽度：紧凑间距，保证可用（F3.3） */
@media (min-width: 768px) and (max-width: 1100px) {
  .main {
    padding: var(--fp-space-4);
  }
}
</style>
