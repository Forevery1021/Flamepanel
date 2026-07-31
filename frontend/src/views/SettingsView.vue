<template>
  <div class="view-container">
    <div class="card-header-title">
      <h2>{{ t('settings.title') }}</h2>
    </div>

    <el-row :gutter="16">
      <el-col :span="12">
        <el-card shadow="hover">
          <template #header
            ><span class="font-semibold">{{ t('settings.changePassword') }}</span></template
          >
          <el-form ref="pwFormRef" :model="pwForm" :label-width="labelWidth" :rules="pwRules">
            <el-form-item :label="t('settings.oldPassword')" prop="old_password">
              <el-input v-model="pwForm.old_password" type="password" show-password />
            </el-form-item>
            <el-form-item :label="t('settings.newPassword')" prop="new_password">
              <el-input v-model="pwForm.new_password" type="password" show-password />
            </el-form-item>
            <el-form-item :label="t('settings.confirmPassword')" prop="confirm">
              <el-input v-model="pwForm.confirm" type="password" show-password />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" :loading="pwSubmitting" @click="handleChangePassword">{{
                t('common.save')
              }}</el-button>
            </el-form-item>
          </el-form>
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card shadow="hover">
          <template #header
            ><span class="font-semibold">{{ t('settings.panelInfo') }}</span></template
          >
          <el-descriptions :column="1" border>
            <el-descriptions-item :label="t('settings.version')">{{
              version
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('settings.panelName')">{{
              settingsMap['panel_name'] || 'FlamePanel'
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('settings.username')">{{
              auth.username
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('settings.role')">
              <el-tag size="small" :type="auth.role === 'admin' ? 'danger' : 'info'">{{
                auth.role
              }}</el-tag>
            </el-descriptions-item>
            <el-descriptions-item :label="t('settings.backend')"
              ><span class="status-ok">Connected</span></el-descriptions-item
            >
          </el-descriptions>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" class="mt-4">
      <template #header
        ><span class="font-semibold">{{ t('settings.panelConfig') }}</span></template
      >
      <el-form v-loading="loading" :model="settingsForm" :label-width="labelWidth">
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item :label="t('settings.panelName')">
              <el-input v-model="settingsForm.panel_name" placeholder="FlamePanel" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('settings.theme')">
              <el-select v-model="settingsForm.theme" class="full-width">
                <el-option :label="t('settings.light')" value="light" />
                <el-option :label="t('settings.dark')" value="dark" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item :label="t('settings.language')">
              <el-select
                v-model="settingsForm.language"
                class="full-width"
                @change="handleLangChange"
              >
                <el-option label="简体中文" value="zh-CN" />
                <el-option label="English" value="en-US" />
                <el-option label="日本語" value="ja-JP" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('settings.panelPort')">
              <el-input-number
                v-model="settingsForm.panel_port_num"
                :min="1024"
                :max="65535"
                class="full-width"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item :label="t('settings.sessionTimeout')">
              <el-input-number
                v-model="settingsForm.session_timeout_num"
                :min="5"
                :max="43200"
                class="full-width"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('settings.logLevel')">
              <el-select v-model="settingsForm.log_level" class="full-width">
                <el-option label="Trace" value="trace" />
                <el-option label="Debug" value="debug" />
                <el-option label="Info" value="info" />
                <el-option label="Warn" value="warn" />
                <el-option label="Error" value="error" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item :label="t('settings.logRetention')">
              <el-input-number
                v-model="settingsForm.log_retention_num"
                :min="1"
                :max="365"
                class="full-width"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('settings.twoFactor')">
              <el-switch v-model="settingsForm.two_factor_enabled_bool" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item>
          <el-button type="primary" :loading="saving" @click="handleSaveSettings">{{
            t('settings.save')
          }}</el-button>
          <el-button @click="resetSettings">{{ t('common.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="mt-4">
      <template #header
        ><span class="font-semibold">JWT {{ t('settings.jwtSecret') }}</span></template
      >
      <el-descriptions :column="1" border>
        <el-descriptions-item :label="t('settings.jwtSecret')">
          <el-tag type="warning" size="small">{{ t('settings.alreadySet') }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item :label="t('common.operation')">
          <el-button type="warning" :loading="rotating" @click="handleRotateJwtSecret">{{
            t('settings.rotate')
          }}</el-button>
          <span class="ml-2 text-xs text-muted">{{ t('common.confirmAction') }}</span>
        </el-descriptions-item>
      </el-descriptions>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { changePassword } from '@/api/auth'
import { listSettings, updateSetting } from '@/api/settings'
import { useAuthStore } from '@/stores/auth'
import { setLanguage } from '@/locales'
import { ElMessage } from 'element-plus'
import { getErrorMessage } from '@/utils/error'
import type { FormInstance, FormRules, FormItemRule } from 'element-plus'

const { t } = useI18n()
const auth = useAuthStore()

const labelWidth = computed(() => (t('settings.panelName').length > 4 ? '140px' : '120px'))
const version = ref('v0.1.0')
const loading = ref(false)
const saving = ref(false)
const pwSubmitting = ref(false)
const rotating = ref(false)

const settingsMap = ref<Record<string, string>>({})
const settingsForm = reactive({
  panel_name: '',
  theme: 'light',
  language: 'zh-CN',
  panel_port_num: 8080,
  session_timeout_num: 1440,
  log_level: 'info',
  log_retention_num: 30,
  two_factor_enabled_bool: false,
})

const pwFormRef = ref<FormInstance>()
const pwForm = ref({ old_password: '', new_password: '', confirm: '' })
const pwRules: FormRules = {
  old_password: [{ required: true, message: t('common.required'), trigger: 'blur' }],
  new_password: [
    { required: true, message: t('common.required'), trigger: 'blur' },
    { min: 6, message: t('settings.passwordLength'), trigger: 'blur' },
  ],
  confirm: [
    { required: true, message: t('common.required'), trigger: 'blur' },
    {
      validator: (_rule: FormItemRule, value: string, callback: (err?: Error) => void) => {
        if (value !== pwForm.value.new_password) callback(new Error(t('settings.passwordMismatch')))
        else callback()
      },
      trigger: 'blur',
    },
  ],
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
    settingsForm.panel_name = map['panel_name'] || 'FlamePanel'
    settingsForm.theme = map['theme'] || 'light'
    settingsForm.language = map['language'] || 'zh-CN'
    settingsForm.panel_port_num = parseInt(map['panel_port'] || '8080')
    settingsForm.session_timeout_num = parseInt(map['session_timeout_minutes'] || '1440')
    settingsForm.log_level = map['log_level'] || 'info'
    settingsForm.log_retention_num = parseInt(map['log_retention_days'] || '30')
    settingsForm.two_factor_enabled_bool = map['two_factor_enabled'] === 'true'
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    loading.value = false
  }
}

function handleLangChange(lang: string) {
  setLanguage(lang)
}

async function handleChangePassword() {
  const valid = await pwFormRef.value?.validate().catch(() => false)
  if (!valid) return
  pwSubmitting.value = true
  try {
    await changePassword(pwForm.value.old_password, pwForm.value.new_password)
    ElMessage.success(t('common.success'))
    pwFormRef.value?.resetFields()
  } catch (e: unknown) {
    ElMessage.error(getErrorMessage(e, t('common.failed')))
  } finally {
    pwSubmitting.value = false
  }
}

async function handleSaveSettings() {
  saving.value = true
  try {
    await updateSetting('panel_name', settingsForm.panel_name)
    await updateSetting('theme', settingsForm.theme)
    await updateSetting('language', settingsForm.language)
    await updateSetting('panel_port', String(settingsForm.panel_port_num))
    await updateSetting('session_timeout_minutes', String(settingsForm.session_timeout_num))
    await updateSetting('log_level', settingsForm.log_level)
    await updateSetting('log_retention_days', String(settingsForm.log_retention_num))
    await updateSetting(
      'two_factor_enabled',
      settingsForm.two_factor_enabled_bool ? 'true' : 'false',
    )
    ElMessage.success(t('common.success'))
    await fetchSettings()
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    saving.value = false
  }
}

function resetSettings() {
  settingsForm.panel_name = settingsMap.value['panel_name'] || 'FlamePanel'
  settingsForm.theme = settingsMap.value['theme'] || 'light'
  settingsForm.language = settingsMap.value['language'] || 'zh-CN'
  settingsForm.panel_port_num = parseInt(settingsMap.value['panel_port'] || '8080')
  settingsForm.session_timeout_num = parseInt(
    settingsMap.value['session_timeout_minutes'] || '1440',
  )
  settingsForm.log_level = settingsMap.value['log_level'] || 'info'
  settingsForm.log_retention_num = parseInt(settingsMap.value['log_retention_days'] || '30')
  settingsForm.two_factor_enabled_bool = settingsMap.value['two_factor_enabled'] === 'true'
  ElMessage.info(t('common.reset'))
}

async function handleRotateJwtSecret() {
  rotating.value = true
  try {
    const secret = Array.from(
      { length: 32 },
      () => 'abcdefghijklmnopqrstuvwxyz0123456789'[Math.floor(Math.random() * 36)],
    ).join('')
    await updateSetting('jwt_secret', secret)
    ElMessage.success(t('common.success'))
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    rotating.value = false
  }
}

onMounted(fetchSettings)
</script>

<style scoped>
.status-ok {
  color: #67c23a;
  font-weight: 600;
}
</style>
