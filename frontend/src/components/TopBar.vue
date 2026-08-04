<template>
  <header class="topbar">
    <div class="topbar-left">
      <el-tooltip :content="t('nav.toggleSidebar')" placement="bottom">
        <el-button v-if="isMobile" text circle class="menu-btn" @click="$emit('open-mobile')">
          <el-icon size="18"><Expand /></el-icon>
        </el-button>
        <el-button v-else text circle class="menu-btn" @click="$emit('toggle-collapse')">
          <el-icon size="18"><Fold v-if="!collapsed" /><Expand v-else /></el-icon>
        </el-button>
      </el-tooltip>
      <div class="topbar-logo" @click="$router.push('/dashboard')">
        <el-icon class="logo-icon"><Lightning /></el-icon>
        <span class="logo-text">FlamePanel</span>
      </div>
      <div class="topbar-divider" />

      <!-- 全局搜索（Ctrl+K） -->
      <el-popover v-model:visible="searchVisible" placement="bottom-start" :width="420" trigger="click">
        <template #reference>
          <div class="search-box" @click="searchVisible = true">
            <el-icon class="search-icon"><Search /></el-icon>
            <input
              v-model="searchText"
              class="search-input"
              :placeholder="t('topbar.searchPlaceholder')"
              @keydown.esc="searchVisible = false"
              @keydown.enter="gotoFirstResult"
            />
            <kbd class="search-kbd">Ctrl K</kbd>
          </div>
        </template>
        <div class="search-panel">
          <div v-if="searchResults.length" class="search-results">
            <div v-for="r in searchResults" :key="r.path" class="search-item" @click="goto(r.path)">
              <el-icon><component :is="r.icon" /></el-icon>
              <span>{{ r.label }}</span>
              <el-tag v-if="r.type === 'app'" size="small" type="warning">{{ t('topbar.app') }}</el-tag>
            </div>
          </div>
          <div v-else-if="searchText" class="search-empty">{{ t('common.noData') }}</div>
          <div v-else class="search-empty">{{ t('topbar.searchHint') }}</div>
        </div>
      </el-popover>
    </div>

    <div class="topbar-right">
      <!-- 快捷应用入口 -->
      <div v-if="quickApps.length" class="quick-apps">
        <el-tooltip
          v-for="app in quickApps"
          :key="app.id"
          :content="app.name"
          placement="bottom"
        >
          <button class="quick-app" @click="openApp(app)">
            <el-icon><Box /></el-icon>
          </button>
        </el-tooltip>
      </div>

      <!-- 实时状态 -->
      <div class="live-status">
        <el-tooltip :content="t('topbar.panelStatus')" placement="bottom">
          <span class="status-item">
            <span class="dot" :class="panelOk ? 'green' : 'red'" />
          </span>
        </el-tooltip>
        <el-tooltip :content="t('topbar.onlineNodes')" placement="bottom">
          <span class="status-item" @click="$router.push('/nodes')">
            <el-icon><OfficeBuilding /></el-icon>
            <span>{{ onlineNodes }}</span>
          </span>
        </el-tooltip>
        <el-tooltip :content="t('topbar.runningContainers')" placement="bottom">
          <span class="status-item" @click="$router.push('/docker')">
            <el-icon><Ship /></el-icon>
            <span>{{ runningContainers }}</span>
          </span>
        </el-tooltip>
      </div>

      <!-- 通知中心（操作日志） -->
      <el-popover placement="bottom-end" :width="320" trigger="click">
        <template #reference>
          <el-button text circle class="notify-btn">
            <el-icon size="18"><Bell /></el-icon>
            <span v-if="notifyCount" class="notify-badge">{{ notifyCount }}</span>
          </el-button>
        </template>
        <div class="notify-panel">
          <div class="notify-header">
            <span class="font-semibold">{{ t('topbar.notifications') }}</span>
            <el-button text size="small" @click="$router.push('/operation-logs')">{{
              t('topbar.viewAll')
            }}</el-button>
          </div>
          <div v-if="notifications.length" class="notify-list">
            <div v-for="n in notifications" :key="n.id" class="notify-item">
              <el-tag
                size="small"
                :type="n.action.startsWith('LOGIN') ? 'warning' : 'info'"
                effect="plain"
              >
                {{ n.action }}
              </el-tag>
              <span class="notify-msg">{{ n.target || '' }}</span>
              <span class="notify-time">{{ shortTime(n.created_at) }}</span>
            </div>
          </div>
          <div v-else class="notify-empty">{{ t('common.noData') }}</div>
        </div>
      </el-popover>

      <el-dropdown trigger="click" @command="handleLangChange">
        <el-button text circle>
          <el-icon size="18"><ChatDotRound /></el-icon>
        </el-button>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item command="zh-CN" :disabled="locale === 'zh-CN'">简体中文</el-dropdown-item>
            <el-dropdown-item command="en-US" :disabled="locale === 'en-US'">English</el-dropdown-item>
            <el-dropdown-item command="ja-JP" :disabled="locale === 'ja-JP'">日本語</el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
      <el-tooltip :content="t('theme.toggle')" placement="bottom">
        <el-button text circle @click="toggleTheme">
          <el-icon size="18"><Moon v-if="isDark" /><Sunny v-else /></el-icon>
        </el-button>
      </el-tooltip>
      <el-dropdown trigger="click" @command="handleCommand">
        <span class="user-dropdown">
          <el-icon><UserFilled /></el-icon>
          <span class="user-name">{{ auth.username }}</span>
          <el-icon class="user-arrow"><ArrowDown /></el-icon>
        </span>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item command="settings">
              <el-icon><Tools /></el-icon> {{ t('nav.settings') }}
            </el-dropdown-item>
            <el-dropdown-item command="logout" divided>
              <el-icon><SwitchButton /></el-icon> {{ t('common.logout') || '退出登录' }}
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>
  </header>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { setLanguage } from '@/locales'
import { applyTheme, isDark as isDarkClass } from '@/utils/theme'
import { listNodes } from '@/api/nodes'
import { listContainers } from '@/api/docker'
import { listInstalledApps, launchApp } from '@/api/appStore'
import type { InstalledApp } from '@/api/appStore'
import { listOperationLogs } from '@/api/logs'
import {
  UserFilled,
  Tools,
  SwitchButton,
  ArrowDown,
  Moon,
  Sunny,
  ChatDotRound,
  Fold,
  Expand,
  Search,
  Bell,
  Box,
  OfficeBuilding,
  Ship,
  Lightning,
} from '@element-plus/icons-vue'


defineProps<{
  collapsed?: boolean
  isMobile?: boolean
}>()
defineEmits<{
  (e: 'toggle-collapse'): void
  (e: 'open-mobile'): void
}>()

const { t, locale } = useI18n()
const router = useRouter()
const auth = useAuthStore()

// ── 实时状态 ──
const panelOk = ref(true)
const onlineNodes = ref(0)
const runningContainers = ref(0)
let statusTimer: number | null = null

async function refreshStatus() {
  try {
    const [nodes, containers] = await Promise.all([
      listNodes(1, 200).catch(() => null),
      listContainers().catch(() => null),
    ])
    onlineNodes.value = nodes?.data.data.filter((n: { status: string }) => n.status === 'online').length ?? 0
    runningContainers.value =
      containers?.data.filter((c: { status: string }) => c.status === 'running').length ?? 0
    panelOk.value = true
  } catch {
    panelOk.value = false
  }
}

// ── 通知中心 ──
const notifications = ref<Array<{ id: number; action: string; target: string | null; created_at: string }>>([])
const notifyCount = computed(() => notifications.value.length)

async function refreshNotifications() {
  try {
    const res = await listOperationLogs(1, 6)
    notifications.value = res.data.data
  } catch {
    notifications.value = []
  }
}

// ── 快捷应用（常用前 4） ──
const installedApps = ref<InstalledApp[]>([])
const quickApps = computed(() =>
  [...installedApps.value].sort((a, b) => (b.launch_count ?? 0) - (a.launch_count ?? 0)).slice(0, 4),
)

async function refreshInstalled() {
  try {
    const res = await listInstalledApps()
    installedApps.value = res.data
  } catch {
    installedApps.value = []
  }
}

async function openApp(app: InstalledApp) {
  // 记录启动次数（常用应用排序）
  launchApp(app.id).catch(() => {})
  if (app.access_url) {
    window.open(app.access_url, '_blank')
  }
}

// ── 全局搜索 ──
const searchVisible = ref(false)
const searchText = ref('')
interface SearchResult {
  path: string
  label: string
  icon: string
  type: 'menu' | 'app'
}
const searchResults = computed<SearchResult[]>(() => {
  const q = searchText.value.trim().toLowerCase()
  if (!q) return []
  const menus: SearchResult[] = [
    { path: '/dashboard', label: t('nav.dashboard'), icon: 'Odometer', type: 'menu' as const },
    { path: '/websites', label: t('nav.websites'), icon: 'Link', type: 'menu' as const },
    { path: '/web-servers', label: t('nav.webServers'), icon: 'Setting', type: 'menu' as const },
    { path: '/app-store', label: t('nav.appStore'), icon: 'Box', type: 'menu' as const },
    { path: '/plugins', label: t('nav.plugins'), icon: 'MagicStick', type: 'menu' as const },
    { path: '/docker', label: t('nav.docker'), icon: 'Ship', type: 'menu' as const },
    { path: '/databases', label: t('nav.databases'), icon: 'Collection', type: 'menu' as const },
    { path: '/files', label: t('nav.files'), icon: 'Folder', type: 'menu' as const },
    { path: '/backups', label: t('nav.backups'), icon: 'Files', type: 'menu' as const },
    { path: '/firewall', label: t('nav.firewall'), icon: 'Lock', type: 'menu' as const },
    { path: '/terminal', label: t('nav.terminal'), icon: 'Monitor', type: 'menu' as const },
    { path: '/nodes', label: t('nav.nodes'), icon: 'OfficeBuilding', type: 'menu' as const },
    { path: '/health', label: t('nav.health'), icon: 'InfoFilled', type: 'menu' as const },
    { path: '/scheduled-tasks', label: t('nav.scheduledTasks'), icon: 'Timer', type: 'menu' as const },
    { path: '/users', label: t('nav.users'), icon: 'User', type: 'menu' as const },
    { path: '/memos', label: t('nav.memos'), icon: 'Notebook', type: 'menu' as const },
    { path: '/operation-logs', label: t('nav.operationLogs'), icon: 'Document', type: 'menu' as const },
    { path: '/system-logs', label: t('nav.systemLogs'), icon: 'List', type: 'menu' as const },
    { path: '/settings', label: t('nav.settings'), icon: 'Tools', type: 'menu' as const },
  ].filter((m) => m.label.toLowerCase().includes(q))
  const apps: SearchResult[] = installedApps.value
    .filter((a: InstalledApp) => a.name.toLowerCase().includes(q))
    .map((a) => ({ path: a.access_url || '/app-store', label: a.name, icon: 'Box', type: 'app' as const }))
  return [...menus, ...apps].slice(0, 10)
})

function goto(path: string) {
  if (path.startsWith('http')) {
    window.open(path, '_blank')
  } else {
    router.push(path)
  }
  searchVisible.value = false
  searchText.value = ''
}

function gotoFirstResult() {
  if (searchResults.value.length) goto(searchResults.value[0].path)
}

function onKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
    e.preventDefault()
    searchVisible.value = true
  }
}

// ── 杂项 ──
const isDark = computed(() => document.documentElement.classList.contains('dark'))

function toggleTheme() {
  applyTheme(isDarkClass() ? 'light' : 'dark')
}

function handleLangChange(lang: string) {
  setLanguage(lang)
}

function handleCommand(cmd: string) {
  if (cmd === 'logout') {
    auth.logout()
    router.push('/login')
  } else if (cmd === 'settings') {
    router.push('/settings')
  }
}

function shortTime(ts: string) {
  const d = new Date(ts)
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
}

onMounted(() => {
  refreshStatus()
  refreshNotifications()
  refreshInstalled()
  statusTimer = window.setInterval(() => {
    refreshStatus()
    refreshNotifications()
  }, 10000)
  window.addEventListener('keydown', onKeydown)
})

onUnmounted(() => {
  if (statusTimer !== null) clearInterval(statusTimer)
  window.removeEventListener('keydown', onKeydown)
})
</script>

<style scoped>
.topbar {
  height: var(--header-height);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 var(--space-4);
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
  gap: var(--space-3);
}
.topbar-left {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  min-width: 0;
  flex: 1;
}
.menu-btn {
  flex-shrink: 0;
}
.topbar-logo {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  flex-shrink: 0;
}
.logo-icon {
  font-size: 20px;
  color: var(--brand);
}
.logo-text {
  font-size: 16px;
  font-weight: 700;
  color: var(--text-primary);
  letter-spacing: 0.5px;
}
.topbar-divider {
  width: 1px;
  height: 20px;
  background: var(--border-strong);
}

/* 搜索 */
.search-box {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 280px;
  max-width: 100%;
  height: 34px;
  padding: 0 10px;
  border-radius: var(--radius-md);
  background: var(--bg-hover);
  border: 1px solid var(--border-color);
  cursor: text;
  transition: border-color var(--transition-fast);
}
.search-box:focus-within {
  border-color: var(--el-color-primary);
}
.search-icon {
  color: var(--text-muted);
  flex-shrink: 0;
}
.search-input {
  flex: 1;
  border: none;
  outline: none;
  background: transparent;
  color: var(--text-primary);
  font-size: 13px;
  min-width: 0;
}
.search-kbd {
  font-size: 11px;
  color: var(--text-muted);
  border: 1px solid var(--border-color);
  border-radius: 4px;
  padding: 1px 5px;
  flex-shrink: 0;
}
.search-panel {
  max-height: 360px;
  overflow: auto;
}
.search-results {
  display: flex;
  flex-direction: column;
}
.search-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: 13px;
  color: var(--text-primary);
}
.search-item:hover {
  background: var(--bg-hover);
}
.search-empty {
  padding: 20px;
  text-align: center;
  color: var(--text-muted);
  font-size: 13px;
}

/* 快捷应用 */
.quick-apps {
  display: flex;
  align-items: center;
  gap: 2px;
  margin-right: var(--space-2);
}
.quick-app {
  width: 32px;
  height: 32px;
  border: none;
  border-radius: var(--radius-md);
  background: var(--bg-hover);
  color: var(--text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 16px;
  transition:
    background-color var(--transition-fast),
    color var(--transition-fast);
}
.quick-app:hover {
  background: color-mix(in srgb, var(--brand) 12%, transparent);
  color: var(--brand);
}

/* 实时状态 */
.live-status {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-right: var(--space-2);
}
.status-item {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 13px;
  color: var(--text-secondary);
  cursor: pointer;
  padding: 4px 6px;
  border-radius: var(--radius-sm);
}
.status-item:hover {
  background: var(--bg-hover);
}
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  display: inline-block;
}
.dot.green {
  background: var(--success);
}
.dot.red {
  background: var(--danger);
}

/* 通知 */
.notify-btn {
  position: relative;
}
.notify-badge {
  position: absolute;
  top: 2px;
  right: 2px;
  min-width: 14px;
  height: 14px;
  border-radius: 7px;
  background: var(--danger);
  color: #fff;
  font-size: 10px;
  line-height: 14px;
  text-align: center;
  padding: 0 3px;
}
.notify-panel {
  max-height: 360px;
  overflow: auto;
}
.notify-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border-color);
}
.notify-list {
  display: flex;
  flex-direction: column;
}
.notify-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 7px 0;
  border-bottom: 1px solid var(--border-color);
  font-size: 12px;
}
.notify-msg {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-secondary);
}
.notify-time {
  color: var(--text-muted);
  flex-shrink: 0;
}
.notify-empty {
  padding: 16px;
  text-align: center;
  color: var(--text-muted);
  font-size: 13px;
}

.topbar-right {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}
.user-dropdown {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  padding: 4px 10px;
  border-radius: var(--radius-md);
  font-size: 14px;
  color: var(--text-secondary);
  transition:
    background-color var(--transition-fast),
    color var(--transition-fast);
}
.user-dropdown:hover {
  background: var(--bg-hover);
}
.user-arrow {
  font-size: 12px;
}
@media (max-width: 768px) {
  .logo-text,
  .topbar-divider,
  .quick-apps,
  .live-status {
    display: none;
  }
  .search-box {
    width: 160px;
  }
}
@media (max-width: 480px) {
  .user-name {
    max-width: 60px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .user-arrow {
    display: none;
  }
}
.dark .topbar {
  background: var(--bg-secondary);
  border-color: var(--border-color);
}
.dark .user-dropdown {
  color: var(--text-secondary);
}
</style>
