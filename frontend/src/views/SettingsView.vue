<template>
  <div>
    <h2>面板设置</h2>
    <el-row :gutter="16" style="margin-top:16px">
      <el-col :span="12">
        <el-card shadow="hover">
          <template #header><span style="font-weight:600">修改密码</span></template>
          <el-form :model="pwForm" label-width="110px" :rules="pwRules" ref="pwFormRef">
            <el-form-item label="当前密码" prop="old_password">
              <el-input v-model="pwForm.old_password" type="password" show-password />
            </el-form-item>
            <el-form-item label="新密码" prop="new_password">
              <el-input v-model="pwForm.new_password" type="password" show-password />
            </el-form-item>
            <el-form-item label="确认密码" prop="confirm">
              <el-input v-model="pwForm.confirm" type="password" show-password />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="handleChangePassword" :loading="pwSubmitting">更新密码</el-button>
            </el-form-item>
          </el-form>
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card shadow="hover">
          <template #header><span style="font-weight:600">面板信息</span></template>
          <el-descriptions :column="1" border>
            <el-descriptions-item label="版本">{{ version }}</el-descriptions-item>
            <el-descriptions-item label="面板名称">{{ settingsMap['panel_name'] || 'FlamePanel' }}</el-descriptions-item>
            <el-descriptions-item label="用户名">{{ auth.username }}</el-descriptions-item>
            <el-descriptions-item label="角色">
              <el-tag size="small" :type="auth.role === 'admin' ? 'danger' : 'info'">{{ auth.role }}</el-tag>
            </el-descriptions-item>
            <el-descriptions-item label="后端状态"><span class="status-ok">Connected</span></el-descriptions-item>
          </el-descriptions>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" style="margin-top:16px">
      <template #header><span style="font-weight:600">面板配置</span></template>
      <el-form :model="settingsForm" label-width="160px" v-loading="loading">
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="面板名称">
              <el-input v-model="settingsForm.panel_name" placeholder="FlamePanel" />
              <div class="setting-hint">显示在页面标题和侧边栏</div>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="主题">
              <el-select v-model="settingsForm.theme" style="width:100%">
                <el-option label="浅色 (Light)" value="light" />
                <el-option label="深色 (Dark)" value="dark" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="界面语言">
              <el-select v-model="settingsForm.language" style="width:100%">
                <el-option label="中文 (简体)" value="zh-CN" />
                <el-option label="English" value="en-US" />
                <el-option label="日本語" value="ja-JP" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="面板端口">
              <el-input-number v-model="settingsForm.panel_port_num" :min="1024" :max="65535" style="width:100%" />
              <div class="setting-hint">修改后需重启面板生效</div>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="会话超时 (分钟)">
              <el-input-number v-model="settingsForm.session_timeout_num" :min="5" :max="43200" style="width:100%" />
              <div class="setting-hint">默认 1440 分钟 (24 小时)</div>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="日志级别">
              <el-select v-model="settingsForm.log_level" style="width:100%">
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
            <el-form-item label="日志保留天数">
              <el-input-number v-model="settingsForm.log_retention_num" :min="1" :max="365" style="width:100%" />
              <div class="setting-hint">超过保留期限的日志将被清理</div>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="两步验证 (2FA)">
              <el-switch v-model="settingsForm.two_factor_enabled_bool" />
              <div class="setting-hint">需要 TOTP 验证器应用</div>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item>
          <el-button type="primary" @click="handleSaveSettings" :loading="saving">保存配置</el-button>
          <el-button @click="resetSettings">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" style="margin-top:16px">
      <template #header><span style="font-weight:600">JWT 密钥</span></template>
      <el-descriptions :column="1" border>
        <el-descriptions-item label="当前密钥">
          <el-tag type="warning" size="small">已设置</el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="操作">
          <el-button type="warning" @click="handleRotateJwtSecret" :loading="rotating">轮换密钥</el-button>
          <span style="margin-left:8px;font-size:12px;color:#909399">轮换后所有用户需要重新登录</span>
        </el-descriptions-item>
      </el-descriptions>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { changePassword } from '@/api/auth'
import { listSettings, updateSetting } from '@/api/settings'
import { useAuthStore } from '@/stores/auth'
import { ElMessage } from 'element-plus'
import type { SettingEntry } from '@/types'
import type { FormInstance, FormRules } from 'element-plus'

const auth = useAuthStore()
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
  old_password: [{ required: true, message: '请输入当前密码', trigger: 'blur' }],
  new_password: [
    { required: true, message: '请输入新密码', trigger: 'blur' },
    { min: 6, message: '至少 6 个字符', trigger: 'blur' },
  ],
  confirm: [
    { required: true, message: '请确认新密码', trigger: 'blur' },
    {
      validator: (_rule: any, value: string, callback: Function) => {
        if (value !== pwForm.value.new_password) callback(new Error('两次密码不一致'))
        else callback()
      }, trigger: 'blur',
    },
  ],
}

async function fetchSettings() {
  loading.value = true
  try {
    const res = await listSettings()
    const map: Record<string, string> = {}
    for (const s of res.data) {
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
  } catch { ElMessage.error('获取配置失败') }
  finally { loading.value = false }
}

async function handleChangePassword() {
  const valid = await pwFormRef.value?.validate().catch(() => false)
  if (!valid) return
  pwSubmitting.value = true
  try {
    await changePassword(pwForm.value.old_password, pwForm.value.new_password)
    ElMessage.success('密码已更新')
    pwFormRef.value?.resetFields()
  } catch (e: any) { ElMessage.error(e.response?.data?.message || '更新失败') }
  finally { pwSubmitting.value = false }
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
    await updateSetting('two_factor_enabled', settingsForm.two_factor_enabled_bool ? 'true' : 'false')
    ElMessage.success('配置已保存')
    await fetchSettings()
  } catch (e: any) { ElMessage.error(e.response?.data?.message || '保存失败') }
  finally { saving.value = false }
}

function resetSettings() {
  settingsForm.panel_name = settingsMap.value['panel_name'] || 'FlamePanel'
  settingsForm.theme = settingsMap.value['theme'] || 'light'
  settingsForm.language = settingsMap.value['language'] || 'zh-CN'
  settingsForm.panel_port_num = parseInt(settingsMap.value['panel_port'] || '8080')
  settingsForm.session_timeout_num = parseInt(settingsMap.value['session_timeout_minutes'] || '1440')
  settingsForm.log_level = settingsMap.value['log_level'] || 'info'
  settingsForm.log_retention_num = parseInt(settingsMap.value['log_retention_days'] || '30')
  settingsForm.two_factor_enabled_bool = settingsMap.value['two_factor_enabled'] === 'true'
  ElMessage.info('已重置为当前保存值')
}

async function handleRotateJwtSecret() {
  rotating.value = true
  try {
    const secret = Array.from({ length: 32 }, () =>
      'abcdefghijklmnopqrstuvwxyz0123456789'[Math.floor(Math.random() * 36)]
    ).join('')
    await updateSetting('jwt_secret', secret)
    ElMessage.success('JWT 密钥已轮换，所有用户需要重新登录')
  } catch (e: any) { ElMessage.error(e.response?.data?.message || '轮换失败') }
  finally { rotating.value = false }
}

onMounted(fetchSettings)
</script>

<style scoped>
.status-ok { color: #67c23a; font-weight: 600; }
.setting-hint { font-size: 12px; color: #909399; margin-top: 4px; }
</style>
