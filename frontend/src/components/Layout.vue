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
      </main>
      <AppFooter />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed, onMounted, onUnmounted } from 'vue'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import Drawer from 'openvue/drawer'
import AppSidebar from './layout/AppSidebar.vue'
import AppHeader from './layout/AppHeader.vue'
import AppFooter from './layout/AppFooter.vue'
import AppTabs from './layout/AppTabs.vue'
import { useTabsStore } from '@/stores/tabs'
import { useThemeStore } from '@/stores/theme'
import { useAppearanceStore } from '@/stores/appearance'

const COLLAPSE_KEY = 'flamepanel.sidebar.collapsed'

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
const collapsed = ref(
  localStorage.getItem(COLLAPSE_KEY) === '1' || appearance.state.menuCollapsed,
)
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
    localStorage.setItem(COLLAPSE_KEY, v ? '1' : '0')
  },
)

function toggleCollapse() {
  const next = !collapsed.value
  collapsed.value = next
  localStorage.setItem(COLLAPSE_KEY, next ? '1' : '0')
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
      localStorage.setItem(COLLAPSE_KEY, '1')
    } else if (w >= 1200 && collapsed.value && localStorage.getItem(COLLAPSE_KEY) !== '1') {
      collapsed.value = false
      localStorage.setItem(COLLAPSE_KEY, '0')
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
</style>
