<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { useTheme, type ThemeColor } from '@/composables/useTheme'
import { useI18n } from 'vue-i18n'
import { setLocale, getLocale } from '@/i18n'
import api from '@/api/client'
import type { PanelSettings } from '@/types'
import { useAuthStore } from '@/stores/auth'
import { Upload, Sunny, Moon, Delete, Plus } from '@element-plus/icons-vue'

const { t } = useI18n()
const {
  currentTheme, currentColor, backgroundImage, backgroundOpacity,
  applyColor, applyBackground, removeBackground, toggleTheme
} = useTheme()
const auth = useAuthStore()

const settings = ref<PanelSettings>({ theme: 'light', language: 'zh-CN' })
const currentLanguage = ref(getLocale())

// Change password form
const passwordForm = ref({
  current_password: '',
  new_password: '',
  confirm_password: '',
})
const changingPassword = ref(false)
const bgUrlInput = ref('')
const bgOpacity = ref(backgroundOpacity.value)
const uploadingBg = ref(false)

const colorPresets: { name: ThemeColor; label: string; cssVar: string }[] = [
  { name: 'blue',   label: '天蓝',  cssVar: '211, 100%, 55%' },
  { name: 'green',  label: '翠绿',  cssVar: '160, 80%, 45%' },
  { name: 'purple', label: '魅紫',  cssVar: '270, 70%, 55%' },
  { name: 'orange', label: '暖橙',  cssVar: '22, 95%, 52%' },
  { name: 'red',    label: '赤红',  cssVar: '350, 85%, 52%' },
  { name: 'cyan',   label: '青蓝',  cssVar: '190, 75%, 48%' },
]

const fetchSettings = async () => {
  try {
    const res = await api.get<PanelSettings>('/settings')
    settings.value = res.data
  } catch { /* defaults */ }
}

const handleChangePassword = async () => {
  if (!passwordForm.value.current_password || !passwordForm.value.new_password) {
    ElMessage.warning(t('settings.fillAllFields'))
    return
  }
  if (passwordForm.value.new_password !== passwordForm.value.confirm_password) {
    ElMessage.warning(t('settings.passwordMismatch'))
    return
  }
  if (passwordForm.value.new_password.length < 6) {
    ElMessage.warning(t('settings.passwordTooShort'))
    return
  }
  changingPassword.value = true
  try {
    await api.put('/auth/change-password', {
      current_password: passwordForm.value.current_password,
      new_password: passwordForm.value.new_password,
    })
    ElMessage.success(t('settings.passwordSuccess'))
    passwordForm.value = { current_password: '', new_password: '', confirm_password: '' }
    auth.logout()
    window.location.href = '/login'
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || t('settings.passwordFailed'))
  } finally {
    changingPassword.value = false
  }
}

const handleThemeToggle = () => {
  toggleTheme()
}

const handleColorChange = (color: ThemeColor) => {
  applyColor(color)
  api.put('/settings', { theme_color: color }).catch(() => {})
}

const handleLanguageChange = (lang: string) => {
  currentLanguage.value = lang
  setLocale(lang)
  api.put('/settings', { language: lang }).catch(() => {})
  window.location.reload()
}

const handleApplyBgUrl = () => {
  const url = bgUrlInput.value.trim()
  if (!url) {
    removeBackground()
    api.put('/settings', { background_image: '', background_opacity: 0 }).catch(() => {})
  } else {
    applyBackground(url, bgOpacity.value)
    api.put('/settings', { background_image: url, background_opacity: bgOpacity.value }).catch(() => {})
  }
}

const handleBgFileUpload = () => {
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = 'image/*'
  input.onchange = async () => {
    const file = input.files?.[0]
    if (!file) return
    uploadingBg.value = true
    try {
      const reader = new FileReader()
      reader.onload = () => {
        const dataUrl = reader.result as string
        applyBackground(dataUrl, bgOpacity.value)
        api.put('/settings', {
          background_image: dataUrl,
          background_opacity: bgOpacity.value
        }).catch(() => {})
        bgUrlInput.value = dataUrl.substring(0, 60) + '...'
        uploadingBg.value = false
        ElMessage.success('背景图片已上传')
      }
      reader.readAsDataURL(file)
    } catch {
      uploadingBg.value = false
      ElMessage.error('上传失败')
    }
  }
  input.click()
}

const handleRemoveBg = () => {
  bgUrlInput.value = ''
  removeBackground()
  api.put('/settings', { background_image: '', background_opacity: 0 }).catch(() => {})
  ElMessage.success('背景图片已移除')
}

const handleOpacityChange = (val: number) => {
  bgOpacity.value = val
  if (backgroundImage.value) {
    applyBackground(backgroundImage.value, val)
    api.put('/settings', { background_opacity: val }).catch(() => {})
  }
}

onMounted(() => {
  fetchSettings()
  if (backgroundImage.value) {
    bgUrlInput.value = backgroundImage.value.length > 80
      ? backgroundImage.value.substring(0, 80) + '...'
      : backgroundImage.value
  }
})
</script>

<template>
  <div class="settings-page">
    <div class="page-header">
      <h2>{{ t('settings.title') }}</h2>
      <p class="desc">{{ t('settings.desc') }}</p>
    </div>

    <div class="settings-sections">
      <!-- Appearance -->
      <el-card class="settings-card">
        <template #header>
          <div class="card-header">
            <span class="card-title">{{ t('settings.appearance') }}</span>
          </div>
        </template>

        <!-- Dark/Light Toggle -->
        <div class="setting-row">
          <div class="setting-info">
            <span class="setting-label">{{ t('settings.theme') }}</span>
            <span class="setting-desc">{{ t('settings.themeDesc') }}</span>
          </div>
          <div class="theme-toggle-group">
            <button
              class="theme-toggle-btn"
              :class="{ active: currentTheme === 'light' }"
              @click="currentTheme !== 'light' && handleThemeToggle()"
            >
              <el-icon><Sunny /></el-icon>
              <span>{{ t('settings.light') }}</span>
            </button>
            <button
              class="theme-toggle-btn"
              :class="{ active: currentTheme === 'dark' }"
              @click="currentTheme !== 'dark' && handleThemeToggle()"
            >
              <el-icon><Moon /></el-icon>
              <span>{{ t('settings.dark') }}</span>
            </button>
          </div>
        </div>

        <!-- Language -->
        <div class="setting-row">
          <div class="setting-info">
            <span class="setting-label">{{ t('settings.language') }}</span>
            <span class="setting-desc">{{ t('settings.languageDesc') }}</span>
          </div>
          <el-select
            :model-value="currentLanguage"
            size="small"
            style="width: 140px"
            @change="handleLanguageChange"
          >
            <el-option label="简体中文" value="zh-CN" />
            <el-option label="English" value="en-US" />
          </el-select>
        </div>

        <!-- Theme Colors -->
        <div class="setting-row setting-row-column">
          <div class="setting-info">
            <span class="setting-label">主题颜色</span>
            <span class="setting-desc">选择界面主色调，全局生效</span>
          </div>
          <div class="color-presets">
            <button
              v-for="c in colorPresets"
              :key="c.name"
              class="color-swatch"
              :class="{ active: currentColor === c.name }"
              :style="{ backgroundColor: `hsl(${c.cssVar})` }"
              :title="c.label"
              @click="handleColorChange(c.name)"
            >
              <span v-if="currentColor === c.name" class="check-mark">&#10003;</span>
            </button>
          </div>
        </div>
      </el-card>

      <!-- Background Image -->
      <el-card class="settings-card">
        <template #header>
          <div class="card-header">
            <span class="card-title">自定义背景</span>
          </div>
        </template>

        <div class="setting-row">
          <div class="setting-info">
            <span class="setting-label">背景图片</span>
            <span class="setting-desc">设置个性化背景，支持图片链接或本地文件</span>
          </div>
        </div>

        <div class="bg-input-row">
          <el-input
            v-model="bgUrlInput"
            placeholder="输入图片 URL..."
            size="default"
            class="bg-url-input"
            clearable
            @clear="handleRemoveBg"
            @keyup.enter="handleApplyBgUrl"
          >
            <template #append>
              <el-button @click="handleApplyBgUrl">应用</el-button>
            </template>
          </el-input>
          <el-button :icon="Upload" :loading="uploadingBg" @click="handleBgFileUpload">
            本地上传
          </el-button>
          <el-button
            v-if="backgroundImage"
            :icon="Delete"
            type="danger"
            plain
            @click="handleRemoveBg"
          >
            移除
          </el-button>
        </div>

        <div v-if="backgroundImage" class="setting-row">
          <div class="setting-info">
            <span class="setting-label">背景透明度</span>
            <span class="setting-desc">{{ Math.round(bgOpacity * 100) }}%</span>
          </div>
          <el-slider
            :model-value="bgOpacity"
            :min="0.1"
            :max="0.8"
            :step="0.05"
            style="width: 200px"
            @input="handleOpacityChange"
          />
        </div>

        <div v-if="backgroundImage" class="bg-preview">
          <img :src="backgroundImage" alt="背景预览" />
        </div>
      </el-card>

      <!-- Security -->
      <el-card class="settings-card">
        <template #header>
          <div class="card-header">
            <span class="card-title">{{ t('settings.security') }}</span>
          </div>
        </template>

        <div class="setting-row">
          <div class="setting-info">
            <span class="setting-label">{{ t('settings.account') }}</span>
            <span class="setting-desc">{{ t('settings.accountDesc') }}</span>
          </div>
          <el-tag>{{ auth.username || 'admin' }}</el-tag>
        </div>

        <div class="setting-row">
          <div class="setting-info">
            <span class="setting-label">{{ t('settings.role') }}</span>
            <span class="setting-desc">{{ t('settings.roleDesc') }}</span>
          </div>
          <el-tag :type="auth.role === 'admin' ? 'danger' : 'info'">
            {{ auth.role === 'admin' ? t('settings.admin') : t('settings.user') }}
          </el-tag>
        </div>

        <el-divider />

        <div class="password-section">
          <h4>{{ t('settings.changePassword') }}</h4>
          <el-form :model="passwordForm" label-width="100px" class="password-form">
            <el-form-item :label="t('settings.currentPassword')">
              <el-input
                v-model="passwordForm.current_password"
                type="password"
                show-password
                :placeholder="t('settings.passwordPlaceholder')"
              />
            </el-form-item>
            <el-form-item :label="t('settings.newPassword')">
              <el-input
                v-model="passwordForm.new_password"
                type="password"
                show-password
                :placeholder="t('settings.newPasswordPlaceholder')"
              />
            </el-form-item>
            <el-form-item :label="t('settings.confirmPassword')">
              <el-input
                v-model="passwordForm.confirm_password"
                type="password"
                show-password
                :placeholder="t('settings.confirmPasswordPlaceholder')"
              />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" :loading="changingPassword" @click="handleChangePassword">
                {{ t('settings.passwordBtn') }}
              </el-button>
            </el-form-item>
          </el-form>
        </div>
      </el-card>

      <!-- About -->
      <el-card class="settings-card">
        <template #header>
          <div class="card-header">
            <span class="card-title">{{ t('settings.about') }}</span>
          </div>
        </template>

        <div class="about-grid">
          <div class="about-item">
            <span class="about-label">{{ t('settings.panelName') }}</span>
            <span class="about-value">Flamepanel</span>
          </div>
          <div class="about-item">
            <span class="about-label">{{ t('settings.version') }}</span>
            <span class="about-value">v0.1.0</span>
          </div>
          <div class="about-item">
            <span class="about-label">{{ t('settings.backend') }}</span>
            <span class="about-value">Rust + Axum 0.8</span>
          </div>
          <div class="about-item">
            <span class="about-label">{{ t('settings.frontend') }}</span>
            <span class="about-value">Vue 3.5 + Element Plus</span>
          </div>
          <div class="about-item">
            <span class="about-label">{{ t('settings.database') }}</span>
            <span class="about-value">SQLite</span>
          </div>
          <div class="about-item">
            <span class="about-label">{{ t('settings.license') }}</span>
            <span class="about-value">MIT</span>
          </div>
        </div>
      </el-card>
    </div>
  </div>
</template>

<style scoped>
.settings-page {
  max-width: 840px;
}

.page-header h2 {
  margin: 0;
  font-size: 22px;
  color: var(--text-primary);
  font-weight: 700;
}
.page-header .desc {
  margin: 4px 0 0;
  color: var(--text-secondary);
  font-size: 13px;
}

.settings-sections {
  margin-top: 24px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.settings-card {
  background: var(--bg-card);
  backdrop-filter: var(--glass-blur);
  -webkit-backdrop-filter: var(--glass-blur);
  border: 1px solid var(--glass-border);
  border-radius: var(--border-radius-lg);
  box-shadow: var(--shadow-card);
}

.settings-card :deep(.el-card__header) {
  background: transparent;
  border-color: var(--border-color);
  padding: 16px 20px;
}

.settings-card :deep(.el-card__body) {
  padding: 20px;
}

.card-header { display: flex; align-items: center; justify-content: space-between; }
.card-title { font-weight: 600; color: var(--text-primary); font-size: 15px; }

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 0;
}
.setting-row + .setting-row { border-top: 1px solid var(--border-light); }
.setting-row-column { flex-direction: column; align-items: flex-start; gap: 12px; }

.setting-info { display: flex; flex-direction: column; gap: 2px; }
.setting-label { font-size: 14px; color: var(--text-primary); font-weight: 500; }
.setting-desc { font-size: 12px; color: var(--text-secondary); }

/* Theme toggle buttons */
.theme-toggle-group {
  display: flex;
  background: var(--bg-hover);
  border-radius: 10px;
  padding: 3px;
  gap: 2px;
}

.theme-toggle-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 7px 16px;
  border: none;
  border-radius: 8px;
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-secondary);
  background: transparent;
  transition: all 0.2s ease;
}
.theme-toggle-btn:hover { color: var(--text-primary); }
.theme-toggle-btn.active {
  background: var(--bg-card);
  color: var(--el-color-primary);
  box-shadow: var(--shadow-sm);
}

/* Color presets */
.color-presets {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.color-swatch {
  width: 42px;
  height: 42px;
  border-radius: 50%;
  border: 3px solid transparent;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s ease;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
}
.color-swatch:hover { transform: scale(1.12); box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2); }
.color-swatch.active { border-color: var(--text-primary); transform: scale(1.08); }

.check-mark { color: #fff; font-size: 16px; font-weight: 700; text-shadow: 0 1px 2px rgba(0,0,0,0.3); }

/* Background */
.bg-input-row { display: flex; gap: 10px; align-items: center; margin-top: 8px; }
.bg-url-input { flex: 1; }

.bg-preview {
  margin-top: 16px;
  border-radius: var(--border-radius);
  overflow: hidden;
  border: 1px solid var(--border-color);
  max-height: 200px;
}
.bg-preview img {
  width: 100%;
  height: 180px;
  object-fit: cover;
  display: block;
}

/* Password */
.password-section h4 { margin: 0 0 16px; color: var(--text-primary); }
.password-form { max-width: 420px; }

/* About */
.about-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
.about-item { display: flex; flex-direction: column; gap: 2px; padding: 8px 0; }
.about-label { font-size: 12px; color: var(--text-secondary); }
.about-value { font-size: 14px; color: var(--text-primary); font-weight: 500; }
</style>
