<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { useTheme } from '@/composables/useTheme'
import api from '@/api/client'
import type { PanelSettings } from '@/types'
import { useAuthStore } from '@/stores/auth'

const { currentTheme, toggleTheme } = useTheme()
const auth = useAuthStore()

const settings = ref<PanelSettings>({ theme: 'light', language: 'zh-CN' })
const loading = ref(false)

// Change password form
const passwordForm = ref({
  current_password: '',
  new_password: '',
  confirm_password: '',
})
const changingPassword = ref(false)

const fetchSettings = async () => {
  try {
    const res = await api.get<PanelSettings>('/settings')
    settings.value = res.data
  } catch {
    // defaults
  }
}

const handleChangePassword = async () => {
  if (!passwordForm.value.current_password || !passwordForm.value.new_password) {
    ElMessage.warning('请填写完整信息')
    return
  }
  if (passwordForm.value.new_password !== passwordForm.value.confirm_password) {
    ElMessage.warning('两次输入的新密码不一致')
    return
  }
  if (passwordForm.value.new_password.length < 6) {
    ElMessage.warning('新密码长度至少 6 位')
    return
  }
  changingPassword.value = true
  try {
    await api.put('/auth/change-password', {
      current_password: passwordForm.value.current_password,
      new_password: passwordForm.value.new_password,
    })
    ElMessage.success('密码修改成功，请重新登录')
    passwordForm.value = { current_password: '', new_password: '', confirm_password: '' }
    auth.logout()
    window.location.href = '/login'
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '密码修改失败')
  } finally {
    changingPassword.value = false
  }
}

const handleThemeToggle = () => {
  toggleTheme()
  // Sync to server immediately
  api.put('/settings', { theme: currentTheme.value === 'dark' ? 'dark' : 'light' }).catch(() => {})
}

onMounted(() => {
  fetchSettings()
})
</script>

<template>
  <div class="settings-page">
    <div class="page-header">
      <h2>面板设置</h2>
      <p class="desc">管理面板外观和账号安全</p>
    </div>

    <div class="settings-sections">
      <!-- Appearance -->
      <el-card class="settings-card">
        <template #header>
          <div class="card-header">
            <span class="card-title">外观设置</span>
          </div>
        </template>

        <div class="setting-row">
          <div class="setting-info">
            <span class="setting-label">主题模式</span>
            <span class="setting-desc">切换浅色 / 深色主题</span>
          </div>
          <el-switch
            :model-value="currentTheme === 'dark'"
            size="large"
            inline-prompt
            active-text="深色"
            inactive-text="浅色"
            @change="handleThemeToggle"
          />
        </div>

        <div class="setting-row">
          <div class="setting-info">
            <span class="setting-label">界面语言</span>
            <span class="setting-desc">当前仅支持简体中文</span>
          </div>
          <el-tag type="info">简体中文</el-tag>
        </div>
      </el-card>

      <!-- Security -->
      <el-card class="settings-card">
        <template #header>
          <div class="card-header">
            <span class="card-title">安全设置</span>
          </div>
        </template>

        <div class="setting-row">
          <div class="setting-info">
            <span class="setting-label">登录账号</span>
            <span class="setting-desc">当前登录的账号名称</span>
          </div>
          <el-tag>{{ auth.username || 'admin' }}</el-tag>
        </div>

        <div class="setting-row">
          <div class="setting-info">
            <span class="setting-label">用户角色</span>
            <span class="setting-desc">当前账号的权限级别</span>
          </div>
          <el-tag :type="auth.role === 'admin' ? 'danger' : 'info'">
            {{ auth.role === 'admin' ? '管理员' : '普通用户' }}
          </el-tag>
        </div>

        <el-divider />

        <div class="password-section">
          <h4>修改密码</h4>
          <el-form :model="passwordForm" label-width="100px" class="password-form">
            <el-form-item label="当前密码">
              <el-input
                v-model="passwordForm.current_password"
                type="password"
                show-password
                placeholder="输入当前密码"
              />
            </el-form-item>
            <el-form-item label="新密码">
              <el-input
                v-model="passwordForm.new_password"
                type="password"
                show-password
                placeholder="输入新密码（至少 6 位）"
              />
            </el-form-item>
            <el-form-item label="确认密码">
              <el-input
                v-model="passwordForm.confirm_password"
                type="password"
                show-password
                placeholder="再次输入新密码"
              />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" :loading="changingPassword" @click="handleChangePassword">
                修改密码
              </el-button>
            </el-form-item>
          </el-form>
        </div>
      </el-card>

      <!-- About -->
      <el-card class="settings-card">
        <template #header>
          <div class="card-header">
            <span class="card-title">关于</span>
          </div>
        </template>

        <div class="about-grid">
          <div class="about-item">
            <span class="about-label">面板名称</span>
            <span class="about-value">Flamepanel</span>
          </div>
          <div class="about-item">
            <span class="about-label">版本</span>
            <span class="about-value">v0.1.0</span>
          </div>
          <div class="about-item">
            <span class="about-label">后端</span>
            <span class="about-value">Rust + Axum 0.8</span>
          </div>
          <div class="about-item">
            <span class="about-label">前端</span>
            <span class="about-value">Vue 3.5 + Element Plus</span>
          </div>
          <div class="about-item">
            <span class="about-label">数据库</span>
            <span class="about-value">SQLite</span>
          </div>
          <div class="about-item">
            <span class="about-label">许可证</span>
            <span class="about-value">MIT</span>
          </div>
        </div>
      </el-card>
    </div>
  </div>
</template>

<style scoped>
.settings-page {
  max-width: 800px;
}

.page-header h2 {
  margin: 0;
  font-size: 22px;
  color: var(--text-primary);
}
.page-header .desc {
  margin: 4px 0 0;
  color: var(--text-secondary);
  font-size: 13px;
}

.settings-sections {
  margin-top: 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.settings-card {
  background: var(--bg-card);
  border-color: var(--border-color);
}
.settings-card :deep(.el-card__header) {
  background: var(--bg-sidebar);
  border-color: var(--border-color);
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.card-title {
  font-weight: 600;
  color: var(--text-primary);
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 0;
}
.setting-row + .setting-row {
  border-top: 1px solid var(--border-light);
}

.setting-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.setting-label {
  font-size: 14px;
  color: var(--text-primary);
  font-weight: 500;
}
.setting-desc {
  font-size: 12px;
  color: var(--text-secondary);
}

.about-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}
.about-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 0;
}
.about-label {
  font-size: 12px;
  color: var(--text-secondary);
}
.about-value {
  font-size: 14px;
  color: var(--text-primary);
  font-weight: 500;
}

.password-section h4 {
  margin: 0 0 16px;
  color: var(--text-primary);
}
.password-form {
  max-width: 420px;
}
</style>
