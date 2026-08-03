<template>
  <header class="topbar">
    <div class="topbar-left">
      <el-tooltip :content="t('nav.toggleSidebar')" placement="bottom">
        <el-button
          v-if="isMobile"
          text
          circle
          class="menu-btn"
          @click="$emit('open-mobile')"
        >
          <el-icon size="18"><Expand /></el-icon>
        </el-button>
        <el-button v-else text circle class="menu-btn" @click="$emit('toggle-collapse')">
          <el-icon size="18"><Fold v-if="!collapsed" /><Expand v-else /></el-icon>
        </el-button>
      </el-tooltip>
      <el-breadcrumb separator="/" class="topbar-breadcrumb">
        <el-breadcrumb-item>{{ breadcrumb }}</el-breadcrumb-item>
      </el-breadcrumb>
    </div>
    <div class="topbar-right">
      <el-dropdown trigger="click" @command="handleLangChange">
        <el-button text circle>
          <el-icon size="18"><ChatDotRound /></el-icon>
        </el-button>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item command="zh-CN" :disabled="locale === 'zh-CN'"
              >简体中文</el-dropdown-item
            >
            <el-dropdown-item command="en-US" :disabled="locale === 'en-US'"
              >English</el-dropdown-item
            >
            <el-dropdown-item command="ja-JP" :disabled="locale === 'ja-JP'"
              >日本語</el-dropdown-item
            >
          </el-dropdown-menu>
        </template>
      </el-dropdown>
      <el-tooltip :content="t('theme.toggle')" placement="bottom">
        <el-button text circle @click="toggleTheme">
          <el-icon size="18"><Moon v-if="isDark" /><Sunny v-else /></el-icon>
        </el-button>
      </el-tooltip>
      <el-tooltip :content="t('health.title')" placement="bottom">
        <el-button text circle @click="$router.push('/health')">
          <el-icon size="18"><InfoFilled /></el-icon>
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
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { setLanguage } from '@/locales'
import { applyTheme, isDark as isDarkClass } from '@/utils/theme'
import {
  UserFilled,
  Tools,
  SwitchButton,
  InfoFilled,
  ArrowDown,
  Moon,
  Sunny,
  ChatDotRound,
  Fold,
  Expand,
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
const route = useRoute()
const router = useRouter()
const auth = useAuthStore()

const routeName = computed(() => (route.name as string) || '')
const breadcrumb = computed(() => {
  const key = `nav.${routeName.value}`
  const label = t(key)
  return label === key ? routeName.value : label
})

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
}
.topbar-left {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
}
.menu-btn {
  flex-shrink: 0;
}
.topbar-breadcrumb {
  font-size: 14px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.topbar-right {
  display: flex;
  align-items: center;
  gap: 4px;
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
@media (max-width: 480px) {
  .user-name {
    max-width: 72px;
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
.dark .user-dropdown:hover {
  background: var(--bg-hover);
}
</style>
