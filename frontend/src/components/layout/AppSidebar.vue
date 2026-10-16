<template>
  <aside class="app-sidebar" :class="{ 'is-collapsed': collapsed }">
    <div
      class="sidebar-logo"
      role="link"
      tabindex="0"
      :aria-label="t('nav.dashboard')"
      @click="router.push('/dashboard')"
      @keydown.enter="router.push('/dashboard')"
    >
      <span class="logo-mark" aria-hidden="true">
        <svg viewBox="0 0 24 24" width="22" height="22" fill="none">
          <path
            d="M12 2c1 3.5-1 5.5-3 8-1.8 2.2-2.5 4-2 6.5A5.5 5.5 0 0 0 12 22a5.5 5.5 0 0 0 5-5.5c.5-2.5-.2-4.3-2-6.5-2-2.5-4-4.5-3-8Z"
            fill="currentColor"
            opacity="0.9"
          />
          <circle cx="12" cy="16.5" r="2" fill="var(--fp-bg-sidebar)" />
        </svg>
      </span>
      <span v-if="!collapsed" class="logo-text">FlamePanel</span>
    </div>

    <nav class="sidebar-nav" :aria-label="t('nav.mainNav')" data-tour="sidebar">
      <!-- 顶级项（main 分组，非钉底） -->
      <template v-for="group in visibleGroups" :key="group.key">
        <template v-if="group.key === 'main'">
          <RouterLink
            v-for="item in group.items.filter((i) => !i.meta.pinned)"
            :key="item.path"
            :to="item.path"
            class="nav-item"
            :class="{ 'is-active': isActive(item.path), 'is-collapsed': collapsed }"
            :title="collapsed ? t(item.meta.title ?? '') : undefined"
            @mouseenter="prefetch.enter(item.path)"
            @mouseleave="prefetch.leave()"
            @focus="prefetch.now(item.path)"
          >
            <span class="nav-item__icon" :class="`menu-color-${group.key}`">
              <i class="oi" :class="item.meta.icon" />
            </span>
            <span v-if="!collapsed" class="nav-item__label">{{ t(item.meta.title ?? '') }}</span>
          </RouterLink>
        </template>

        <!-- 二级折叠分组 -->
        <template v-else>
          <div v-if="!collapsed" class="nav-group">
            <button
              class="nav-group__title"
              :class="{ 'is-open': appearance.isGroupOpen(group.key) }"
              @click="appearance.toggleGroup(group.key)"
            >
              <span class="nav-group__icon" :class="`menu-color-${group.key}`">
                <i class="oi" :class="group.icon" />
              </span>
              <span class="nav-group__label">{{ t(group.labelKey) }}</span>
              <i class="oi oi-angle-down nav-group__arrow" />
            </button>
            <transition name="submenu">
              <div v-if="appearance.isGroupOpen(group.key)" class="nav-submenu">
                <RouterLink
                  v-for="item in group.items"
                  :key="item.path"
                  :to="item.path"
                  class="nav-item nav-item-sub"
                  :class="{ 'is-active': isActive(item.path) }"
                  @mouseenter="prefetch.enter(item.path)"
                  @mouseleave="prefetch.leave()"
                  @focus="prefetch.now(item.path)"
                >
                  <span class="nav-item__dot" aria-hidden="true" />
                  <span class="nav-item__label">{{ t(item.meta.title ?? '') }}</span>
                </RouterLink>
              </div>
            </transition>
          </div>

          <!-- 折叠态：分组图标 + 点击弹出浮层 -->
          <div v-else class="nav-group-collapsed">
            <button
              class="nav-group__btn"
              :aria-label="t(group.labelKey)"
              :title="t(group.labelKey)"
              @click="toggleFlyout(group.key, $event)"
            >
              <span class="nav-group__icon" :class="`menu-color-${group.key}`" aria-hidden="true">
                <i class="oi" :class="group.icon" />
              </span>
            </button>
          </div>
        </template>
      </template>
    </nav>

    <!-- UX-3：钉底一级菜单（面板设置）——始终可达，不随分组折叠/隐藏 -->
    <div v-if="pinnedBottom.length" class="sidebar-pinned" data-tour="settings">
      <RouterLink
        v-for="item in pinnedBottom"
        :key="item.path"
        :to="item.path"
        class="nav-item nav-item-pinned"
        :class="{ 'is-active': isActive(item.path), 'is-collapsed': collapsed }"
        :title="collapsed ? t(item.meta.title ?? '') : undefined"
        @mouseenter="prefetch.enter(item.path)"
        @mouseleave="prefetch.leave()"
        @focus="prefetch.now(item.path)"
      >
        <span class="nav-item__icon menu-color-system">
          <i class="oi" :class="item.meta.icon" />
        </span>
        <span v-if="!collapsed" class="nav-item__label">{{ t(item.meta.title ?? '') }}</span>
        <kbd v-if="!collapsed" class="nav-item__kbd" aria-hidden="true">,</kbd>
      </RouterLink>
    </div>

    <!-- 折叠态子菜单浮层 -->
    <div
      v-if="flyout"
      class="nav-flyout"
      :style="{ top: flyoutTop + 'px' }"
      @mouseleave="flyout = null"
    >
      <div class="nav-flyout__title">{{ t(flyout.labelKey) }}</div>
      <RouterLink
        v-for="item in flyout.items"
        :key="item.path"
        :to="item.path"
        class="nav-item nav-item-sub"
        :class="{ 'is-active': isActive(item.path) }"
        @mouseenter="prefetch.enter(item.path)"
        @mouseleave="prefetch.leave()"
        @focus="prefetch.now(item.path)"
        @click="flyout = null"
      >
        <span class="nav-item__icon" :class="`menu-color-${flyout.key}`">
          <i class="oi" :class="item.meta.icon" />
        </span>
        <span class="nav-item__label">{{ t(item.meta.title ?? '') }}</span>
      </RouterLink>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAppearanceStore } from '@/stores/appearance'
import { useAuthStore } from '@/stores/auth'
import { menuRoutes, type MenuGroup } from '@/router'
import { isRouteVisibleInMenu } from '@/directives/permission'
import { useHoverPrefetch } from '@/utils/prefetchRoute'

defineProps<{ collapsed?: boolean }>()

const router = useRouter()
// 悬停 / 聚焦菜单项时预取目标视图 chunk
const prefetch = useHoverPrefetch(router)
const route = useRoute()
const { t } = useI18n()
const appearance = useAppearanceStore()
const auth = useAuthStore()

// Modernization M7：按角色默认折叠次要分组（仅当用户未手动自定义分组时生效）
appearance.applyRoleDefaults(auth.role)

const GROUP_LABELS: Record<MenuGroup, string> = {
  main: '',
  web: 'nav.groupWeb',
  apps: 'nav.groupApps',
  storage: 'nav.groupStorage',
  ops: 'nav.groupOps',
  system: 'nav.groupSystem',
}

/** 分组彩色图标（语义色圆底） */
const GROUP_ICONS: Record<MenuGroup, string> = {
  main: 'oi-gauge',
  web: 'oi-globe',
  apps: 'oi-box',
  storage: 'oi-database',
  ops: 'oi-shield',
  system: 'oi-cog',
}

/** 按分组聚合路由表（meta.group + 未隐藏 + 角色可见） */
const visibleGroups = computed(() => {
  const role = auth.role
  const byGroup = new Map<MenuGroup, typeof menuRoutes>()
  for (const r of menuRoutes) {
    const group = r.meta?.group
    if (!group || appearance.state.hideMenu.includes(group)) continue
    // 按角色过滤菜单（与路由守卫同源判据：权限点 + 角色级菜单规则）
    if (!isRouteVisibleInMenu(role, r)) continue
    const list = byGroup.get(group) ?? []
    list.push(r)
    byGroup.set(group, list)
  }
  return [...byGroup.entries()].map(([key, items]) => ({
    key,
    labelKey: GROUP_LABELS[key],
    icon: GROUP_ICONS[key],
    items,
  }))
})

/** 钉在导航底部的一级项（`meta.pinned === 'bottom'`，目前为「面板设置」） */
const pinnedBottom = computed(
  () => visibleGroups.value.find((g) => g.key === 'main')?.items.filter((i) => i.meta.pinned === 'bottom') ?? [],
)

/** 快捷键：`,`（逗号，macOS 偏好设置惯例）直达面板设置；输入框聚焦时不劫持 */
function onGlobalKeydown(e: KeyboardEvent) {
  if (e.key !== ',' || e.ctrlKey || e.metaKey || e.altKey) return
  const target = e.target as HTMLElement | null
  if (target && (target.isContentEditable || /^(input|textarea|select)$/i.test(target.tagName))) return
  const settings = pinnedBottom.value[0]
  if (!settings) return
  e.preventDefault()
  if (route.path !== settings.path) router.push(settings.path)
}
onMounted(() => window.addEventListener('keydown', onGlobalKeydown))
onUnmounted(() => window.removeEventListener('keydown', onGlobalKeydown))

/** 进入/切换路由时自动展开所在分组（不覆盖用户手动收起：只在未打开时打开） */
watch(
  () => route.path,
  () => {
    const g = route.meta?.group as MenuGroup | undefined
    if (g && g !== 'main') appearance.ensureGroupOpen(g)
  },
  { immediate: true },
)

/** 高亮：路由路径或 meta.activeMenu 归属 */
function isActive(path: string) {
  if (route.path === path) return true
  return route.meta?.activeMenu === path
}

// ── 折叠态浮层 ──
interface Flyout {
  key: MenuGroup
  labelKey: string
  icon: string
  items: typeof menuRoutes
}
const flyout = ref<Flyout | null>(null)
const flyoutTop = ref(0)

function toggleFlyout(key: MenuGroup, event: MouseEvent) {
  if (flyout.value?.key === key) {
    flyout.value = null
    return
  }
  const group = visibleGroups.value.find((g) => g.key === key)
  if (!group) return
  const btn = event.currentTarget as HTMLElement
  flyoutTop.value = Math.min(btn.offsetTop, 400)
  flyout.value = {
    key,
    labelKey: group.labelKey,
    icon: group.icon,
    items: group.items,
  }
}
</script>

<style scoped>
.app-sidebar {
  position: relative;
  display: flex;
  flex-direction: column;
  width: var(--fp-sidebar-width);
  height: 100%;
  background: var(--fp-bg-sidebar);
  border-right: 1px solid var(--fp-border);
  transition: width var(--fp-transition-base);
  overflow: hidden;
}
.app-sidebar.is-collapsed {
  width: var(--fp-sidebar-width-collapsed);
}

.sidebar-logo {
  display: flex;
  align-items: center;
  gap: 10px;
  height: var(--fp-header-height);
  padding: 0 18px;
  border-bottom: 1px solid oklch(1 0 0 / 0.07);
  cursor: pointer;
  flex-shrink: 0;
}
.app-sidebar.is-collapsed .sidebar-logo {
  justify-content: center;
  padding: 0;
}
.logo-mark {
  color: var(--fp-brand);
  display: flex;
  align-items: center;
  filter: drop-shadow(0 0 8px var(--fp-brand-soft));
}
.logo-text {
  font-size: 16px;
  font-weight: 700;
  letter-spacing: 0.02em;
  color: oklch(0.98 0 0);
  white-space: nowrap;
}

.sidebar-nav {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: var(--fp-space-2);
  display: flex;
  flex-direction: column;
  gap: 2px;
}

/* ── 彩色分组图标（语义色圆底） ── */
.menu-color-main {
  color: var(--fp-brand);
  background: var(--fp-brand-soft);
}
.menu-color-web {
  color: oklch(0.65 0.16 155);
  background: oklch(0.65 0.16 155 / 0.14);
}
.menu-color-apps {
  color: oklch(0.6 0.18 300);
  background: oklch(0.6 0.18 300 / 0.14);
}
.menu-color-storage {
  color: oklch(0.6 0.15 245);
  background: oklch(0.6 0.15 245 / 0.14);
}
.menu-color-ops {
  color: oklch(0.6 0.21 25);
  background: oklch(0.6 0.21 25 / 0.14);
}
.menu-color-system {
  color: oklch(0.62 0.02 250);
  background: oklch(0.62 0.02 250 / 0.14);
}
.nav-item__icon,
.nav-group__icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: 7px;
  font-size: 12.5px;
  flex-shrink: 0;
  transition:
    background-color var(--fp-transition-fast),
    color var(--fp-transition-fast);
}
.nav-item.is-active .nav-item__icon {
  color: var(--fp-brand);
  background: var(--fp-brand-soft);
}

/* ── 顶级项 ── */
.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  height: 38px;
  padding: 0 10px;
  border-radius: var(--fp-radius-sm);
  color: oklch(0.75 0.02 250);
  font-size: 13.5px;
  text-decoration: none;
  transition:
    background-color var(--fp-transition-fast),
    color var(--fp-transition-fast);
}
.nav-item:hover {
  background: oklch(1 0 0 / 0.06);
  color: oklch(0.95 0 0);
}
.nav-item {
  position: relative;
}
.nav-item.is-active {
  background: oklch(1 0 0 / 0.05);
  color: oklch(0.97 0 0);
  font-weight: 600;
}
.nav-item.is-active::before {
  content: '';
  position: absolute;
  left: -8px;
  top: 8px;
  bottom: 8px;
  width: 3px;
  border-radius: 0 3px 3px 0;
  background: var(--fp-brand);
}
.nav-item-sub.is-active::before {
  left: -16px;
}
.nav-item__label {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.nav-item.is-collapsed {
  justify-content: center;
  padding: 0;
  width: 44px;
  margin: 0 auto;
}

/* ── 二级分组 ── */
.nav-group {
  display: flex;
  flex-direction: column;
}
.nav-group__title {
  display: flex;
  align-items: center;
  gap: 10px;
  height: 40px;
  padding: 0 10px;
  border: none;
  border-radius: var(--fp-radius-sm);
  background: transparent;
  color: oklch(0.72 0.02 250);
  font-size: 13px;
  font-weight: 600;
  font-family: var(--fp-font-sans);
  margin-top: 4px;
  cursor: pointer;
  text-align: left;
  width: 100%;
  transition:
    background-color var(--fp-transition-fast),
    color var(--fp-transition-fast);
}
.nav-group__title:hover {
  background: oklch(1 0 0 / 0.06);
  color: oklch(0.95 0 0);
}
.nav-group__title.is-open {
  color: oklch(0.95 0 0);
}
.nav-group__label {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.nav-group__arrow {
  font-size: 10px;
  color: oklch(0.6 0.02 250);
  transition: transform var(--fp-transition-fast);
}
.nav-group__title.is-open .nav-group__arrow {
  transform: rotate(180deg);
}

.nav-submenu {
  display: flex;
  flex-direction: column;
  gap: 1px;
  padding: 2px 0 6px var(--fp-space-4);
  margin-left: 19px;
  border-left: 1px solid oklch(1 0 0 / 0.08);
}
.nav-submenu .nav-item__dot { display: none; }
.nav-item-sub {
  height: 34px;
  font-size: 13px;
}
.nav-item__dot {
  width: 4px;
  height: 4px;
  border-radius: 999px;
  background: oklch(0.55 0.02 250);
  flex-shrink: 0;
  margin-left: 2px;
  transition: background-color var(--fp-transition-fast);
}
.nav-item-sub:hover .nav-item__dot {
  background: oklch(0.9 0 0);
}
.nav-item-sub.is-active .nav-item__dot {
  background: var(--fp-brand);
}
.nav-item-sub.is-active {
  background: oklch(1 0 0 / 0.05);
  color: oklch(0.97 0 0);
}

/* 子菜单展开过渡 */
.submenu-enter-active,
.submenu-leave-active {
  transition:
    opacity var(--fp-transition-fast),
    transform var(--fp-transition-fast);
}
.submenu-enter-from,
.submenu-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

/* ── 钉底一级菜单 ── */
.sidebar-pinned {
  flex-shrink: 0;
  padding: var(--fp-space-2);
  border-top: 1px solid oklch(1 0 0 / 0.07);
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.nav-item-pinned .nav-item__label {
  flex: 1;
}
.nav-item__kbd {
  font-family: var(--fp-font-mono);
  font-size: 10px;
  line-height: 1;
  padding: 3px 5px;
  border-radius: 4px;
  border: 1px solid oklch(1 0 0 / 0.12);
  color: oklch(0.6 0.02 250);
  opacity: 0;
  transition: opacity var(--fp-transition-fast);
}
.nav-item-pinned:hover .nav-item__kbd {
  opacity: 1;
}

/* ── 折叠态分组按钮 ── */
.nav-group-collapsed {
  display: flex;
  justify-content: center;
  padding: 2px 0;
}
.nav-group__btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border: none;
  border-radius: var(--fp-radius-sm);
  background: transparent;
  cursor: pointer;
  transition: background-color var(--fp-transition-fast);
}
.nav-group__btn:hover {
  background: oklch(1 0 0 / 0.06);
}

/* ── 折叠态浮层 ── */
.nav-flyout {
  position: absolute;
  left: calc(var(--fp-sidebar-width-collapsed) + 6px);
  min-width: 180px;
  padding: var(--fp-space-2);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
  box-shadow: var(--fp-shadow-lg);
  z-index: var(--fp-z-sidebar);
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.nav-flyout__title {
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--fp-text-muted);
  padding: var(--fp-space-2) var(--fp-space-3) var(--fp-space-1);
}
.nav-flyout .nav-item {
  color: var(--fp-text-primary);
}
.nav-flyout .nav-item:hover {
  background: var(--fp-bg-hover);
  color: var(--fp-text-primary);
}
.nav-flyout .nav-item.is-active {
  background: var(--fp-brand-soft);
  color: var(--fp-brand);
}
</style>
