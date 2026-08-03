<template>
  <el-menu
    :default-active="route.path"
    :collapse="collapsed"
    :collapse-transition="false"
    router
    class="sidebar"
  >
    <div v-if="!collapsed" class="sidebar-logo">
      <el-icon class="logo-icon"><Lightning /></el-icon>
      <span class="logo-text">FlamePanel</span>
    </div>
    <div v-else class="sidebar-logo collapsed-logo">
      <el-icon class="logo-icon"><Lightning /></el-icon>
    </div>

    <el-menu-item index="/dashboard">
      <el-icon><Odometer /></el-icon>
      <template #title>{{ t('nav.dashboard') }}</template>
    </el-menu-item>

    <!-- 网站 -->
    <el-sub-menu index="web">
      <template #title>
        <el-icon><Link /></el-icon>
        <span>{{ t('nav.groupWeb') }}</span>
      </template>
      <el-menu-item index="/websites">
        <el-icon><Link /></el-icon>
        <span>{{ t('nav.websites') }}</span>
      </el-menu-item>
      <el-menu-item index="/web-servers">
        <el-icon><Setting /></el-icon>
        <span>{{ t('nav.webServers') }}</span>
      </el-menu-item>
    </el-sub-menu>

    <!-- 应用 -->
    <el-sub-menu index="apps">
      <template #title>
        <el-icon><Box /></el-icon>
        <span>{{ t('nav.groupApps') }}</span>
      </template>
      <el-menu-item index="/app-store">
        <el-icon><Box /></el-icon>
        <span>{{ t('nav.appStore') }}</span>
      </el-menu-item>
      <el-menu-item index="/plugins">
        <el-icon><MagicStick /></el-icon>
        <span>{{ t('nav.plugins') }}</span>
      </el-menu-item>
    </el-sub-menu>

    <!-- 容器与存储 -->
    <el-sub-menu index="storage">
      <template #title>
        <el-icon><Ship /></el-icon>
        <span>{{ t('nav.groupStorage') }}</span>
      </template>
      <el-menu-item index="/docker">
        <el-icon><Ship /></el-icon>
        <span>{{ t('nav.docker') }}</span>
      </el-menu-item>
      <el-menu-item index="/databases">
        <el-icon><Collection /></el-icon>
        <span>{{ t('nav.databases') }}</span>
      </el-menu-item>
      <el-menu-item index="/files">
        <el-icon><Folder /></el-icon>
        <span>{{ t('nav.files') }}</span>
      </el-menu-item>
      <el-menu-item index="/backups">
        <el-icon><Files /></el-icon>
        <span>{{ t('nav.backups') }}</span>
      </el-menu-item>
    </el-sub-menu>

    <!-- 安全与运维 -->
    <el-sub-menu index="ops">
      <template #title>
        <el-icon><Lock /></el-icon>
        <span>{{ t('nav.groupOps') }}</span>
      </template>
      <el-menu-item index="/firewall">
        <el-icon><Lock /></el-icon>
        <span>{{ t('nav.firewall') }}</span>
      </el-menu-item>
      <el-menu-item index="/terminal">
        <el-icon><Monitor /></el-icon>
        <span>{{ t('nav.terminal') }}</span>
      </el-menu-item>
      <el-menu-item index="/nodes">
        <el-icon><OfficeBuilding /></el-icon>
        <span>{{ t('nav.nodes') }}</span>
      </el-menu-item>
      <el-menu-item index="/health">
        <el-icon><InfoFilled /></el-icon>
        <span>{{ t('nav.health') }}</span>
      </el-menu-item>
      <el-menu-item index="/scheduled-tasks">
        <el-icon><Timer /></el-icon>
        <span>{{ t('nav.scheduledTasks') }}</span>
      </el-menu-item>
    </el-sub-menu>

    <!-- 系统 -->
    <el-sub-menu index="system">
      <template #title>
        <el-icon><Setting /></el-icon>
        <span>{{ t('nav.groupSystem') }}</span>
      </template>
      <el-menu-item index="/users">
        <el-icon><User /></el-icon>
        <span>{{ t('nav.users') }}</span>
      </el-menu-item>
      <el-menu-item index="/operation-logs">
        <el-icon><Document /></el-icon>
        <span>{{ t('nav.operationLogs') }}</span>
      </el-menu-item>
      <el-menu-item index="/system-logs">
        <el-icon><List /></el-icon>
        <span>{{ t('nav.systemLogs') }}</span>
      </el-menu-item>
      <el-menu-item index="/settings">
        <el-icon><Tools /></el-icon>
        <span>{{ t('nav.settings') }}</span>
      </el-menu-item>
    </el-sub-menu>
  </el-menu>
</template>

<script setup lang="ts">
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import {
  Odometer,
  Link,
  Ship,
  Setting,
  Document,
  List,
  Tools,
  InfoFilled,
  Folder,
  Collection,
  Files,
  Timer,
  Lock,
  Box,
  User,
  Monitor,
  MagicStick,
  OfficeBuilding,
  Lightning,
} from '@element-plus/icons-vue'

defineProps<{ collapsed?: boolean }>()

const { t } = useI18n()
const route = useRoute()
</script>

<style scoped>
.sidebar {
  width: 100%;
  height: 100%;
  border-right: 1px solid var(--border-color);
  overflow-y: auto;
  overflow-x: hidden;
  background: var(--bg-sidebar);
  --el-menu-bg-color: var(--bg-sidebar);
  --el-menu-text-color: #94a3b8;
  --el-menu-hover-bg-color: rgb(255 255 255 / 0.06);
  --el-menu-active-color: #fff;
  --el-menu-item-height: 42px;
  --el-menu-sub-item-height: 38px;
  --el-menu-base-level-padding: 18px;
}
.sidebar:not(.el-menu--collapse) {
  width: var(--sidebar-width);
}
.sidebar.el-menu--collapse {
  width: 64px;
}
.sidebar .el-menu {
  border-right: none;
}

.sidebar-logo {
  display: flex;
  align-items: center;
  gap: 10px;
  height: var(--header-height);
  padding: 0 18px;
  border-bottom: 1px solid rgb(255 255 255 / 0.06);
}
.collapsed-logo {
  justify-content: center;
  padding: 0;
}
.logo-icon {
  font-size: 22px;
  color: var(--brand);
}
.logo-text {
  font-size: 16px;
  font-weight: 700;
  color: #fff;
  letter-spacing: 0.5px;
  white-space: nowrap;
}

.sidebar .el-menu-item:hover,
.sidebar .el-sub-menu__title:hover {
  background: rgb(255 255 255 / 0.06);
}
.sidebar .el-menu-item.is-active {
  background: var(--el-color-primary);
  border-radius: var(--radius-sm);
  margin: 2px var(--space-2);
  width: calc(100% - var(--space-4));
}
.sidebar .el-menu-item,
.sidebar .el-sub-menu__title {
  border-radius: var(--radius-sm);
  margin: 2px var(--space-2);
  width: calc(100% - var(--space-4));
}
.sidebar.el-menu--collapse .el-menu-item,
.sidebar.el-menu--collapse .el-sub-menu__title {
  width: auto;
  margin: 4px;
}
.dark .sidebar {
  border-color: var(--border-color);
}
</style>
