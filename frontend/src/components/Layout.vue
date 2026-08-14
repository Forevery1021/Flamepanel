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
      <AppTabs v-if="appearance.state.menuTabs" :is-mobile="isMobile" />
      <main class="main">
        <router-view v-slot="{ Component }">
          <transition name="page" mode="out-in">
            <keep-alive :include="tabsStore.cachedNames">
              <component :is="Component" />
            </keep-alive>
          </transition>
        </router-view>
        <span ref="mainHeadingRef" tabindex="-1" class="sr-only" />
      </main>
      <AppFooter />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useStorage } from '@vueuse/core'
import Drawer from 'openvue/drawer'
import AppSidebar from './layout/AppSidebar.vue'
import AppHeader from './layout/AppHeader.vue'
import AppFooter from './layout/AppFooter.vue'
import AppTabs from './layout/AppTabs.vue'
import { useTabsStore } from '@/stores/tabs'
import { useThemeStore } from '@/stores/theme'
import { useAppearanceStore } from '@/stores/appearance'
import { STORAGE_KEYS, rawBooleanSerializer } from '@/utils/storage'

const route = useRoute()
const { t } = useI18n()
const tabsStore = useTabsStore()
const themeStore = useThemeStore()
const appearance = useAppearanceStore()
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
const isMobile = ref(false)
const mobileOpen = ref(false)
let mql: MediaQueryList | null = null

// 路由变化 → 打开页签（首页固定）
watch(
  () => route.path,
  (path) => {
    if (path.startsWith('/login')) return
    tabsStore.open(path, route.meta?.title ? t(route.meta.title) : path)
  },
  { immediate: true },
)

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

function syncMobile(e: MediaQueryListEvent | MediaQueryList) {
  isMobile.value = e.matches
  if (e.matches) mobileOpen.value = false
}

// <1200px 自动折叠侧边栏（1Panel 风格响应式）
let resizeTimer: number | null = null
function onResize() {
  if (isMobile.value) return
  if (resizeTimer !== null) window.clearTimeout(resizeTimer)
  resizeTimer = window.setTimeout(() => {
    const w = window.innerWidth
    if (w < 1200 && !collapsed.value) {
      collapsed.value = true
    } else if (w >= 1200 && collapsed.value) {
      collapsed.value = false
    }
  }, 150)
}

onMounted(() => {
  mql = window.matchMedia('(max-width: 768px)')
  syncMobile(mql)
  mql.addEventListener('change', syncMobile)
  window.addEventListener('resize', onResize)
})

onUnmounted(() => {
  mql?.removeEventListener('change', syncMobile)
  window.removeEventListener('resize', onResize)
  if (resizeTimer !== null) window.clearTimeout(resizeTimer)
})

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
  },
)

watch(
  () => route.fullPath,
  () => {
    if (isMobile.value) mobileOpen.value = false
  },
)
</script>

<style scoped>
.layout {
  position: relative;
  display: flex;
  height: 100vh;
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
  background: var(--fp-bg-app);
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

@media (max-width: 768px) {
  .main {
    padding: var(--fp-space-3);
  }
}

/* 平板宽度：紧凑间距，保证可用（F3.3） */
@media (min-width: 769px) and (max-width: 1100px) {
  .main {
    padding: var(--fp-space-4);
  }
}
</style>
