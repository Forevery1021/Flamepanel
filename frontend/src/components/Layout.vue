<template>
  <div class="layout" :class="{ 'is-collapsed': collapsed, 'is-mobile': isMobile }">
    <Sidebar
      v-if="!isMobile"
      :collapsed="collapsed"
      class="layout-sidebar"
    />
    <el-drawer
      v-else
      v-model="mobileOpen"
      direction="ltr"
      size="240px"
      :with-header="false"
      class="mobile-drawer"
    >
      <Sidebar :collapsed="false" />
    </el-drawer>

    <div class="layout-right">
      <TopBar
        :collapsed="collapsed"
        :is-mobile="isMobile"
        @toggle-collapse="toggleCollapse"
        @open-mobile="mobileOpen = true"
      />
      <main class="main">
        <router-view v-slot="{ Component }">
          <transition name="page" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { useRoute } from 'vue-router'
import Sidebar from './Sidebar.vue'
import TopBar from './TopBar.vue'

const COLLAPSE_KEY = 'flamepanel.sidebar.collapsed'

const route = useRoute()
const collapsed = ref(localStorage.getItem(COLLAPSE_KEY) === '1')
const isMobile = ref(false)
const mobileOpen = ref(false)
let mql: MediaQueryList | null = null

function toggleCollapse() {
  collapsed.value = !collapsed.value
  localStorage.setItem(COLLAPSE_KEY, collapsed.value ? '1' : '0')
}

function syncMobile(e: MediaQueryListEvent | MediaQueryList) {
  isMobile.value = e.matches
  if (e.matches) mobileOpen.value = false
}

onMounted(() => {
  mql = window.matchMedia('(max-width: 768px)')
  syncMobile(mql)
  mql.addEventListener('change', syncMobile)
})

onUnmounted(() => {
  mql?.removeEventListener('change', syncMobile)
})

// 移动端切换路由后自动关闭抽屉
watch(
  () => route.fullPath,
  () => {
    if (isMobile.value) mobileOpen.value = false
  },
)
</script>

<style scoped>
.layout {
  display: flex;
  height: 100vh;
  overflow: hidden;
}
.layout-right {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.main {
  flex: 1;
  padding: var(--space-5);
  overflow: auto;
  background: var(--bg-primary);
}

/* 页面切换过渡 */
.page-enter-active,
.page-leave-active {
  transition:
    opacity var(--transition-base),
    transform var(--transition-base);
}
.page-enter-from {
  opacity: 0;
  transform: translateY(6px);
}
.page-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

.mobile-drawer :deep(.el-drawer__body) {
  padding: 0;
}

/* 小屏收窄内容边距 */
@media (max-width: 768px) {
  .main {
    padding: var(--space-3);
  }
}
</style>
