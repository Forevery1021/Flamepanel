<template>
  <div class="login">
    <el-card class="card">
      <div class="login-head">
        <el-icon class="login-logo"><Lightning /></el-icon>
        <h2>FlamePanel</h2>
        <p class="login-sub">{{ t('login.subtitle') }}</p>
      </div>
      <el-form ref="formRef" :model="form" :rules="rules" @submit.prevent="handleLogin">
        <el-form-item prop="username">
          <el-input
            v-model="form.username"
            size="large"
            :placeholder="t('login.username')"
            :prefix-icon="User"
          />
        </el-form-item>
        <el-form-item prop="password">
          <el-input
            v-model="form.password"
            type="password"
            size="large"
            show-password
            :placeholder="t('login.password')"
            :prefix-icon="Lock"
            @keyup.enter="handleLogin"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" native-type="submit" :loading="loading" class="full-width" size="large">
            {{ loading ? t('login.loggingIn') : t('login.login') }}
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- 强制修改密码 -->
    <el-dialog
      v-model="showForceChange"
      :title="t('login.forceChangeTitle')"
      width="420px"
      :close-on-click-modal="false"
      :show-close="false"
      append-to-body
    >
      <el-alert :title="t('login.forceChangeHint')" type="warning" :closable="false" class="mb-3" />
      <el-form
        ref="forceFormRef"
        :model="forceForm"
        :rules="forceRules"
        label-width="100px"
        @submit.prevent="handleForceChange"
      >
        <el-form-item :label="t('settings.oldPassword')" prop="old_password">
          <el-input v-model="forceForm.old_password" type="password" show-password />
        </el-form-item>
        <el-form-item :label="t('settings.newPassword')" prop="new_password">
          <el-input v-model="forceForm.new_password" type="password" show-password />
        </el-form-item>
        <el-form-item :label="t('settings.confirmPassword')" prop="confirm">
          <el-input v-model="forceForm.confirm" type="password" show-password />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button type="primary" :loading="forceSubmitting" @click="handleForceChange">{{
          t('common.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { changePassword } from '@/api/auth'
import { ElMessage } from 'element-plus'
import { User, Lock, Lightning } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'

const { t } = useI18n()
const form = reactive({ username: 'admin', password: 'admin123' })
const formRef = ref<FormInstance>()
const loading = ref(false)
const router = useRouter()
const auth = useAuthStore()

// 强制改密
const showForceChange = ref(false)
const forceSubmitting = ref(false)
const forceFormRef = ref<FormInstance>()
const forceForm = reactive({ old_password: '', new_password: '', confirm: '' })
const forceRules: FormRules = {
  old_password: [{ required: true, message: t('settings.oldPasswordRequired'), trigger: 'blur' }],
  new_password: [
    { required: true, message: t('settings.newPasswordRequired'), trigger: 'blur' },
    { min: 8, message: t('settings.passwordMinLength'), trigger: 'blur' },
  ],
  confirm: [
    {
      validator: (_r, v, cb) =>
        v === forceForm.new_password ? cb() : cb(new Error(t('settings.passwordMismatch'))),
      trigger: 'blur',
    },
  ],
}

const rules: FormRules = {
  username: [{ required: true, message: t('login.usernameRequired'), trigger: 'blur' }],
  password: [{ required: true, message: t('login.passwordRequired'), trigger: 'blur' }],
}

async function handleLogin() {
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return
  loading.value = true
  try {
    const res = await auth.login(form.username, form.password)
    if (res.must_change_password) {
      forceForm.old_password = form.password
      showForceChange.value = true
    } else {
      router.push('/dashboard')
    }
  } catch (e) {
    const err = e as { code?: string; message?: string }
    ElMessage.error(err.code === 'AUTH_FORBIDDEN' ? t('login.locked') : t('login.error'))
  } finally {
    loading.value = false
  }
}

async function handleForceChange() {
  const valid = await forceFormRef.value?.validate().catch(() => false)
  if (!valid) return
  forceSubmitting.value = true
  try {
    await changePassword(forceForm.old_password, forceForm.new_password)
    ElMessage.success(t('common.success'))
    showForceChange.value = false
    router.push('/dashboard')
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    forceSubmitting.value = false
  }
}
</script>

<style scoped>
.login {
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--space-4);
  background:
    radial-gradient(1200px 600px at 20% -10%, color-mix(in srgb, var(--brand) 10%, transparent), transparent),
    var(--bg-primary);
}
.card {
  width: 400px;
  max-width: 100%;
}
.login-head {
  text-align: center;
  margin-bottom: var(--space-6);
}
.login-logo {
  font-size: 34px;
  color: var(--brand);
}
.login-head h2 {
  margin-top: var(--space-2);
  font-size: 22px;
  font-weight: 700;
  color: var(--text-primary);
  letter-spacing: 0.5px;
}
.login-sub {
  margin-top: var(--space-2);
  font-size: 13px;
  color: var(--text-secondary);
}
</style>
