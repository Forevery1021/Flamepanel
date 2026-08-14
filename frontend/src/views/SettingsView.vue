<template>
  <div class="view-container">
    <FpTabs v-model="activeTab" class="settings-tabs" :items="tabItems">
<template #general>
<div class="settings-card">
          <div class="settings-grid">
            <FpInput v-model="settingsForm.panel_name" :label="t('settings.panelName')" />
            <FpSelect
              v-model="settingsForm.language"
              :label="t('settings.language')"
              :options="langOptions"
              option-label="label"
              option-value="value"
              @update:model-value="(v) => handleLangChange(String(v ?? 'zh-CN'))"
            />
            <div class="field-col">
              <label class="field-label">{{ t('settings.panelPort') }}</label>
              <FpNumber
                v-model="settingsForm.panel_port_num"
                :min="1024"
                :max="65535"
                class="w-full"
              />
            </div>
            <div class="field-col">
              <label class="field-label">{{ t('settings.sessionTimeout') }}</label>
              <FpNumber
                v-model="settingsForm.session_timeout_num"
                :min="5"
                :max="43200"
                class="w-full"
              />
            </div>
            <FpSelect
              v-model="settingsForm.log_level"
              :label="t('settings.logLevel')"
              :options="logLevelOptions"
              option-label="label"
              option-value="value"
            />
            <div class="field-col">
              <label class="field-label">{{ t('settings.logRetention') }}</label>
              <FpNumber
                v-model="settingsForm.log_retention_num"
                :min="1"
                :max="365"
                class="w-full"
              />
            </div>
            <div class="field-col field-row">
              <label class="field-label">{{ t('settings.twoFactor') }}</label>
              <FpSwitch v-model="settingsForm.two_factor_enabled_bool" />
            </div>
          </div>
          <div class="settings-actions">
            <FpButton variant="primary" :loading="saving" @click="handleSaveSettings">
              {{ t('settings.save') }}
            </FpButton>
            <FpButton variant="ghost" @click="resetSettings">{{ t('common.reset') }}</FpButton>
          </div>
        </div>
</template>
<template #security>
<div class="settings-card">
          <h3 class="settings-section">{{ t('settings.changePassword') }}</h3>
          <div class="settings-form-col">
            <FpInput v-model="pwForm.old_password" :label="t('settings.oldPassword')" type="password" />
            <FpInput
              v-model="pwForm.new_password"
              :label="t('settings.newPassword')"
              type="password"
              toggle-mask
            />
            <FpInput
              v-model="pwForm.confirm"
              :label="t('settings.confirmPassword')"
              type="password"
              toggle-mask
            />
            <div>
              <FpButton variant="primary" :loading="pwSubmitting" @click="handleChangePassword">
                {{ t('common.save') }}
              </FpButton>
            </div>
          </div>

          <FpDivider />

          <h3 class="settings-section">JWT {{ t('settings.jwtSecret') }}</h3>
          <div class="settings-row">
            <FpTag severity="warning" :value="t('settings.alreadySet')" />
            <FpButton variant="warning" :loading="rotating" @click="handleRotateJwtSecret">
              {{ t('settings.rotate') }}
            </FpButton>
            <span class="hint">{{ t('common.confirmAction') }}</span>
          </div>
        </div>
</template>
<template #backup>
<div class="settings-card">
          <div class="settings-form-col settings-form-col-narrow">
            <div class="field-col field-row">
              <label class="field-label">{{ t('settings.autoBackup') }}</label>
              <FpSwitch v-model="backupForm.enabled" />
              <span class="hint">{{ t('settings.autoBackupHint') }}</span>
            </div>
            <div class="field-col">
              <label class="field-label">{{ t('settings.backupInterval') }}</label>
              <div class="field-inline">
                <FpNumber v-model="backupForm.intervalHours" :min="1" :max="168" style="width: 160px" />
                <span class="hint">{{ t('settings.hoursUnit') }}</span>
              </div>
            </div>
            <div class="field-col">
              <label class="field-label">{{ t('settings.backupRetention') }}</label>
              <div class="field-inline">
                <FpNumber v-model="backupForm.retention" :min="1" :max="100" style="width: 160px" />
                <span class="hint">{{ t('settings.backupRetentionHint') }}</span>
              </div>
            </div>
            <div>
              <FpButton variant="primary" :loading="backupSaving" @click="handleSaveBackupSettings">
                {{ t('common.save') }}
              </FpButton>
            </div>
          </div>
        </div>
</template>
<template #theme>
        <!-- P5：主题设置拆分为独立组件 -->
        <SettingsThemeTab />
      </template>
      <template #appearance>
<div class="settings-card">
          <h3 class="settings-section">{{ t('settingsAppearance.interface') }}</h3>
          <div class="settings-form-col settings-form-col-narrow">
            <div class="field-col field-row">
              <label class="field-label">{{ t('settingsAppearance.menuTabs') }}</label>
              <FpSwitch
                :model-value="appearance.state.menuTabs"
                @update:model-value="toggleMenuTabs"
              />
              <span class="hint">{{ t('settingsAppearance.menuTabsHint') }}</span>
            </div>
            <div class="field-col field-row">
              <label class="field-label">{{ t('settingsAppearance.menuAccordion') }}</label>
              <FpSwitch
                :model-value="appearance.state.menuAccordion"
                @update:model-value="toggleMenuAccordion"
              />
              <span class="hint">{{ t('settingsAppearance.menuAccordionHint') }}</span>
            </div>
            <div class="field-col field-row">
              <label class="field-label">{{ t('settingsAppearance.menuShow') }}</label>
              <FpSelect
                :model-value="appearance.state.hideMenu"
                :options="menuGroupOptions"
                option-label="label"
                option-value="value"
                multiple
                :placeholder="t('settingsAppearance.menuShowHint')"
                class="menu-select"
                @update:model-value="(v) => saveMenuVisibility(v ?? [])"
              />
            </div>
          </div>

          <FpDivider />

          <h3 class="settings-section">{{ t('settingsAppearance.background') }}</h3>
          <div class="settings-form-col settings-form-col-narrow">
            <div class="field-col">
              <label class="field-label">{{ t('settingsAppearance.appBackground') }}</label>
              <div class="field-inline">
                <div v-if="themeStore.custom.appBackground" class="bg-preview" :style="{ backgroundImage: `url(${themeStore.custom.appBackground})` }" />
                <FpButton variant="ghost" icon="oi oi-upload" @click="pickBg('app')">
                  {{ t('settingsAppearance.upload') }}
                </FpButton>
                <FpButton v-if="themeStore.custom.appBackground" variant="link" @click="clearBg('app')">
                  {{ t('settingsAppearance.clear') }}
                </FpButton>
                <span class="hint">{{ t('settingsAppearance.bgHint') }}</span>
              </div>
            </div>
            <div class="field-col">
              <label class="field-label">{{ t('settingsAppearance.loginBackground') }}</label>
              <div class="field-inline">
                <div v-if="themeStore.custom.loginBackground" class="bg-preview" :style="{ backgroundImage: `url(${themeStore.custom.loginBackground})` }" />
                <FpButton variant="ghost" icon="oi oi-upload" @click="pickBg('login')">
                  {{ t('settingsAppearance.upload') }}
                </FpButton>
                <FpButton v-if="themeStore.custom.loginBackground" variant="link" @click="clearBg('login')">
                  {{ t('settingsAppearance.clear') }}
                </FpButton>
                <span class="hint">{{ t('settingsAppearance.bgHint') }}</span>
              </div>
            </div>
          </div>
        </div>
</template>
</FpTabs>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'










import { changePassword } from '@/api/auth'
import { listSettings, updateSetting } from '@/api/settings'
import { setLanguage } from '@/locales'
import { useThemeStore } from '@/stores/theme'
import { useAppearanceStore } from '@/stores/appearance'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import { useFpToast } from '@/components/ui/FpToast'
import FpDivider from '@/components/ui/FpDivider.vue'
import FpNumber from '@/components/ui/FpNumber.vue'
import SettingsThemeTab from '@/components/settings/SettingsThemeTab.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import FpTabs from '@/components/ui/FpTabs.vue'
import type { FpTabItem } from '@/components/ui/FpTabs.vue'

const { t } = useI18n()

const tabItems: FpTabItem[] = [
  { value: 'general', label: t('settings.panelConfig') },
  { value: 'security', label: t('settings.security') },
  { value: 'backup', label: t('settings.backupSettings') },
  { value: 'theme', label: t('settingsTheme.title') },
  { value: 'appearance', label: t('settingsAppearance.title') },
]
const toast = useFpToast()
const themeStore = useThemeStore()
const appearance = useAppearanceStore()

const activeTab = ref('general')
const loading = ref(false)
const saving = ref(false)
const pwSubmitting = ref(false)
const rotating = ref(false)
const backupSaving = ref(false)

const langOptions = [
  { label: '简体中文', value: 'zh-CN' },
  { label: 'English', value: 'en-US' },
  { label: '日本語', value: 'ja-JP' },
]
const logLevelOptions = ['trace', 'debug', 'info', 'warn', 'error'].map((v) => ({
  label: v[0].toUpperCase() + v.slice(1),
  value: v,
}))

const settingsMap = ref<Record<string, string>>({})
const settingsForm = reactive({
  panel_name: '',
  language: 'zh-CN',
  panel_port_num: 8080,
  session_timeout_num: 1440,
  log_level: 'info',
  log_retention_num: 30,
  two_factor_enabled_bool: false,
})

const backupForm = reactive({ enabled: false, intervalHours: 24, retention: 7 })
const pwForm = ref({ old_password: '', new_password: '', confirm: '' })

// ── 外观：多页签 / 菜单显隐 ──
const menuGroupOptions = computed(() => [
  { label: t('nav.groupWeb'), value: 'web' },
  { label: t('nav.groupApps'), value: 'apps' },
  { label: t('nav.groupStorage'), value: 'storage' },
  { label: t('nav.groupOps'), value: 'ops' },
  { label: t('nav.groupSystem'), value: 'system' },
])

async function toggleMenuTabs(value: boolean) {
  appearance.update({ menuTabs: value })
  try {
    await updateSetting('open_menu_tabs', value ? 'true' : 'false')
    toast.success(t('common.success'))
  } catch {
    toast.error(t('common.failed'))
  }
}

async function toggleMenuAccordion(value: boolean) {
  appearance.update({ menuAccordion: value })
  try {
    await updateSetting('menu_accordion', value ? 'true' : 'false')
    toast.success(t('common.success'))
  } catch {
    toast.error(t('common.failed'))
  }
}

async function saveMenuVisibility(hidden: string[] | string | number) {
  const hide = Array.isArray(hidden) ? hidden : []
  appearance.update({ hideMenu: hide })
  try {
    await updateSetting('hide_menu', JSON.stringify(hide))
    toast.success(t('common.success'))
  } catch {
    toast.error(t('common.failed'))
  }
}

// ── 外观：自定义背景 ──
const MAX_BG_BYTES = 500 * 1024

function pickBg(target: 'app' | 'login') {
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = 'image/*'
  input.onchange = async () => {
    const file = input.files?.[0]
    if (!file) return
    try {
      const dataUrl = await compressImage(file)
      applyBackground(target, dataUrl)
    } catch {
      toast.error(t('settingsAppearance.tooLarge'))
    }
  }
  input.click()
}

function compressImage(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    if (file.size > 5 * 1024 * 1024) {
      reject(new Error('too large'))
      return
    }
    const img = new Image()
    img.onload = () => {
      const maxW = 1920
      const scale = Math.min(1, maxW / img.width)
      const canvas = document.createElement('canvas')
      canvas.width = Math.round(img.width * scale)
      canvas.height = Math.round(img.height * scale)
      const ctx = canvas.getContext('2d')
      if (!ctx) {
        reject(new Error('canvas'))
        return
      }
      ctx.drawImage(img, 0, 0, canvas.width, canvas.height)
      let quality = 0.85
      let dataUrl = canvas.toDataURL('image/jpeg', quality)
      while (dataUrl.length > MAX_BG_BYTES && quality > 0.4) {
        quality -= 0.1
        dataUrl = canvas.toDataURL('image/jpeg', quality)
      }
      if (dataUrl.length > MAX_BG_BYTES) {
        reject(new Error('too large'))
        return
      }
      resolve(dataUrl)
    }
    img.onerror = () => reject(new Error('image'))
    img.src = URL.createObjectURL(file)
  })
}

async function applyBackground(target: 'app' | 'login', dataUrl: string) {
  const key = target === 'app' ? 'app_background' : 'login_background'
  themeStore.updateCustom(target === 'app' ? { appBackground: dataUrl } : { loginBackground: dataUrl })
  try {
    await updateSetting(key, dataUrl)
    toast.success(t('common.success'))
  } catch {
    toast.error(t('common.failed'))
  }
}

async function clearBg(target: 'app' | 'login') {
  const key = target === 'app' ? 'app_background' : 'login_background'
  themeStore.updateCustom(target === 'app' ? { appBackground: '' } : { loginBackground: '' })
  try {
    await updateSetting(key, '')
    toast.success(t('common.success'))
  } catch {
    toast.error(t('common.failed'))
  }
}

async function fetchSettings() {
  loading.value = true
  try {
    const res = await listSettings(1, 50)
    const map: Record<string, string> = {}
    for (const s of res.data.data) {
      map[s.key] = s.value
    }
    settingsMap.value = map
    appearance.syncFromServer(map)
    themeStore.syncFromServer(map)
    settingsForm.panel_name = map['panel_name'] || 'FlamePanel'
    settingsForm.language = map['language'] || 'zh-CN'
    settingsForm.panel_port_num = parseInt(map['panel_port'] || '8080')
    settingsForm.session_timeout_num = parseInt(map['session_timeout_minutes'] || '1440')
    settingsForm.log_level = map['log_level'] || 'info'
    settingsForm.log_retention_num = parseInt(map['log_retention_days'] || '30')
    settingsForm.two_factor_enabled_bool = map['two_factor_enabled'] === 'true'
    backupForm.enabled = map['auto_backup_enabled'] === 'true'
    backupForm.intervalHours = parseInt(map['auto_backup_interval_hours'] || '24')
    backupForm.retention = parseInt(map['backup_retention'] || '7')
  } catch {
    toast.error(t('common.failed'))
  } finally {
    loading.value = false
  }
}

function handleLangChange(lang: string) {
  setLanguage(lang)
}

async function handleSaveBackupSettings() {
  backupSaving.value = true
  try {
    await updateSetting('auto_backup_enabled', backupForm.enabled ? 'true' : 'false')
    await updateSetting('auto_backup_interval_hours', String(backupForm.intervalHours))
    await updateSetting('backup_retention', String(backupForm.retention))
    toast.success(t('common.success'))
    await fetchSettings()
  } catch {
    toast.error(t('common.failed'))
  } finally {
    backupSaving.value = false
  }
}

async function handleChangePassword() {
  if (!pwForm.value.old_password || !pwForm.value.new_password || pwForm.value.confirm !== pwForm.value.new_password) {
    toast.warning(t('settings.passwordMismatch'))
    return
  }
  pwSubmitting.value = true
  try {
    await changePassword(pwForm.value.old_password, pwForm.value.new_password)
    toast.success(t('common.success'))
    pwForm.value = { old_password: '', new_password: '', confirm: '' }
  } catch (e: unknown) {
    toast.error(e, t('common.failed'))
  } finally {
    pwSubmitting.value = false
  }
}

async function handleSaveSettings() {
  saving.value = true
  try {
    await updateSetting('panel_name', settingsForm.panel_name)
    await updateSetting('language', settingsForm.language)
    await updateSetting('panel_port', String(settingsForm.panel_port_num))
    await updateSetting('session_timeout_minutes', String(settingsForm.session_timeout_num))
    await updateSetting('log_level', settingsForm.log_level)
    await updateSetting('log_retention_days', String(settingsForm.log_retention_num))
    await updateSetting('two_factor_enabled', settingsForm.two_factor_enabled_bool ? 'true' : 'false')
    toast.success(t('common.success'))
    await fetchSettings()
  } catch {
    toast.error(t('common.failed'))
  } finally {
    saving.value = false
  }
}

function resetSettings() {
  settingsForm.panel_name = settingsMap.value['panel_name'] || 'FlamePanel'
  settingsForm.language = settingsMap.value['language'] || 'zh-CN'
  settingsForm.panel_port_num = parseInt(settingsMap.value['panel_port'] || '8080')
  settingsForm.session_timeout_num = parseInt(settingsMap.value['session_timeout_minutes'] || '1440')
  settingsForm.log_level = settingsMap.value['log_level'] || 'info'
  settingsForm.log_retention_num = parseInt(settingsMap.value['log_retention_days'] || '30')
  settingsForm.two_factor_enabled_bool = settingsMap.value['two_factor_enabled'] === 'true'
  toast.info(t('common.reset'))
}

async function handleRotateJwtSecret() {
  rotating.value = true
  try {
    const secret = Array.from(
      { length: 32 },
      () => 'abcdefghijklmnopqrstuvwxyz0123456789'[Math.floor(Math.random() * 36)],
    ).join('')
    await updateSetting('jwt_secret', secret)
    toast.success(t('common.success'))
  } catch {
    toast.error(t('common.failed'))
  } finally {
    rotating.value = false
  }
}

onMounted(fetchSettings)
</script>

<style scoped>
.settings-tabs {
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-md);
  padding: var(--fp-space-4);
}
.settings-card {
  padding-top: var(--fp-space-2);
  max-width: 900px;
}
.settings-section {
  font-size: 14.5px;
  font-weight: 600;
  color: var(--fp-text-primary);
  margin-bottom: var(--fp-space-4);
}
.settings-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: var(--fp-space-4);
}
.settings-form-col {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
  max-width: 420px;
}
.settings-form-col-narrow {
  max-width: 560px;
}
.settings-row {
  display: flex;
  align-items: center;
  gap: var(--fp-space-3);
}
.settings-actions {
  display: flex;
  gap: var(--fp-space-2);
  margin-top: var(--fp-space-5);
}
.field-col {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.field-row {
  flex-direction: row;
  align-items: center;
  gap: var(--fp-space-3);
}
.field-inline {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
}
.field-label {
  font-size: 13px;
  color: var(--fp-text-secondary);
}
.hint {
  font-size: 12px;
  color: var(--fp-text-muted);
}

.menu-select {
  max-width: 320px;
}
.bg-preview {
  width: 120px;
  height: 64px;
  border-radius: var(--fp-radius-sm);
  border: 1px solid var(--fp-border);
  background-size: cover;
  background-position: center;
  flex-shrink: 0;
}

@media (max-width: 768px) {
  .settings-grid {
    grid-template-columns: 1fr;
  }
}
</style>
