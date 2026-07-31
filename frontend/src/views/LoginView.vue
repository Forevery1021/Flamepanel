<template>
  <div class="login">
    <el-card class="card">
      <h2>FlamePanel</h2>
      <el-form ref="formRef" :model="form" :rules="rules" @submit.prevent="handleLogin">
        <el-form-item :label="t('login.username')" prop="username">
          <el-input
            v-model="form.username"
            :placeholder="t('login.placeholder', { field: t('login.username') })"
          />
        </el-form-item>
        <el-form-item :label="t('login.password')" prop="password">
          <el-input
            v-model="form.password"
            type="password"
            show-password
            :placeholder="t('login.placeholder', { field: t('login.password') })"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" native-type="submit" :loading="loading" class="full-width">
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
import type { FormInstance, FormRules } from 'element-plus'

const { t } = useI18n()
const form = reactive({ username: 'admin', password: 'admin123' })
const formRef = ref<FormInstance>()
const loading = ref(false)
const router = useRouter()
const auth = useAuthStore()

const rules: FormRules = {
  username: [
    {
      required: true,
      message: t('login.placeholder', { field: t('login.username') }),
      trigger: 'blur',
    },
  ],
  password: [
    {
      required: true,
      message: t('login.placeholder', { field: t('login.password') }),
      trigger: 'blur',
    },
  ],
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
  background: var(--bg-primary);
}
.card {
  width: 400px;
}
.card h2 {
  text-align: center;
  margin-bottom: 24px;
  color: var(--brand);
}
</style>
