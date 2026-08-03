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
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { ElMessage } from 'element-plus'
import { User, Lock, Lightning } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'

const { t } = useI18n()
const form = reactive({ username: 'admin', password: 'admin123' })
const formRef = ref<FormInstance>()
const loading = ref(false)
const router = useRouter()
const auth = useAuthStore()

const rules: FormRules = {
  username: [{ required: true, message: t('login.usernameRequired'), trigger: 'blur' }],
  password: [{ required: true, message: t('login.passwordRequired'), trigger: 'blur' }],
}

async function handleLogin() {
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return
  loading.value = true
  try {
    await auth.login(form.username, form.password)
    router.push('/dashboard')
  } catch {
    ElMessage.error(t('login.error'))
  } finally {
    loading.value = false
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
