<template>
  <div class="login">
    <div class="login-bg" aria-hidden="true" />
    <div class="login-glow" aria-hidden="true" />

    <div class="login-container">
      <div class="login-brand" aria-hidden="true">
        <div class="login-brand__inner">
          <span class="logo-mark">
            <svg viewBox="0 0 24 24" width="56" height="56" fill="none">
              <path
                d="M12 2c1 3.5-1 5.5-3 8-1.8 2.2-2.5 4-2 6.5A5.5 5.5 0 0 0 12 22a5.5 5.5 0 0 0 5-5.5c.5-2.5-.2-4.3-2-6.5-2-2.5-4-4.5-3-8Z"
                fill="url(#flameGrad)"
                opacity="0.92"
              />
              <circle cx="12" cy="16.5" r="2" fill="var(--fp-bg-elevated)" />
              <defs>
                <linearGradient id="flameGrad" x1="8" y1="2" x2="16" y2="22">
                  <stop offset="0%" stop-color="var(--fp-brand)" />
                  <stop offset="100%" stop-color="var(--fp-brand-strong)" />
                </linearGradient>
              </defs>
            </svg>
          </span>
          <h1 class="login-brand__name">FlamePanel</h1>
          <p class="login-brand__slogan">{{ t('login.subtitle') }}</p>
          <ul class="login-brand__features">
            <li><i class="oi oi-gauge" />{{ t('login.featMonitor') }}</li>
            <li><i class="oi oi-database" />{{ t('login.featDocker') }}</li>
            <li><i class="oi oi-shield" />{{ t('login.featSecurity') }}</li>
          </ul>
        </div>
      </div>

      <div class="login-form-panel">
        <form class="login-form" @submit.prevent="handleLogin">
          <h2 class="login-form__title">{{ t('login.title') }}</h2>
          <p class="login-form__sub">{{ t('login.welcome') }}</p>
          <FpInput
            v-model="form.username"
            :label="t('login.username')"
            :error="usernameError"
            autocomplete="username"
            @update:model-value="usernameError = ''"
          />
          <FpInput
            v-model="form.password"
            :label="t('login.password')"
            type="password"
            toggle-mask
            :error="passwordError"
            autocomplete="current-password"
            @update:model-value="passwordError = ''"
          />
          <FpButton
            type="submit"
            variant="primary"
            :loading="loading"
            class="login-btn"
            size="large"
          >
            {{ loading ? t('login.loggingIn') : t('login.login') }}
          </FpButton>
        </form>
      </div>
    </div>

    <!-- 强制修改密码 -->
    <FpModal v-model="showForceChange" :header="t('login.forceChangeTitle')" :closable="false">
      <div class="force-alert">
        <i class="oi oi-exclamation-triangle" />
        {{ t('login.forceChangeHint') }}
      </div>
      <div class="force-form">
        <FpInput
          v-model="forceForm.old_password"
          :label="t('settings.oldPassword')"
          type="password"
          :error="forceErrors.old"
        />
        <FpInput
          v-model="forceForm.new_password"
          :label="t('settings.newPassword')"
          type="password"
          toggle-mask
          :error="forceErrors.newPwd"
        />
        <FpInput
          v-model="forceForm.confirm"
          :label="t('settings.confirmPassword')"
          type="password"
          toggle-mask
          :error="forceErrors.confirm"
        />
      </div>
      <template #footer>
        <FpButton variant="primary" :loading="forceSubmitting" @click="handleForceChange">
          {{ t('common.confirm') }}
        </FpButton>
      </template>
    </FpModal>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { changePassword } from '@/api/auth'
import FpInput from '@/components/ui/FpInput.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpModal from '@/components/ui/FpModal.vue'
import { useFpToast } from '@/components/ui/FpToast'

const { t } = useI18n()
const form = reactive({ username: 'admin', password: 'admin123' })
const usernameError = ref('')
const passwordError = ref('')
const loading = ref(false)
const router = useRouter()
const auth = useAuthStore()
const toast = useFpToast()

// 强制改密
const showForceChange = ref(false)
const forceSubmitting = ref(false)
const forceForm = reactive({ old_password: '', new_password: '', confirm: '' })
const forceErrors = reactive({ old: '', newPwd: '', confirm: '' })

// 背景预加载（1Panel preloadImage 模式，防闪烁）
function preloadBackground() {
  const bg = getComputedStyle(document.documentElement).getPropertyValue('--fp-login-bg').trim()
  if (!bg.startsWith('url(')) return
  const url = bg.replace(/^url\("?(.+?)"?\)$/, '$1')
  const img = new Image()
  img.src = url
}

function validateLogin(): boolean {
  usernameError.value = form.username ? '' : t('login.usernameRequired')
  passwordError.value = form.password ? '' : t('login.passwordRequired')
  return !usernameError.value && !passwordError.value
}

async function handleLogin() {
  if (!validateLogin()) return
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
    toast.error(new Error(err.code === 'AUTH_FORBIDDEN' ? t('login.locked') : t('login.error')))
  } finally {
    loading.value = false
  }
}

function validateForce(): boolean {
  forceErrors.old = forceForm.old_password ? '' : t('settings.oldPasswordRequired')
  forceErrors.newPwd = forceForm.new_password
    ? forceForm.new_password.length >= 8
      ? ''
      : t('settings.passwordMinLength')
    : t('settings.newPasswordRequired')
  forceErrors.confirm =
    forceForm.confirm && forceForm.confirm === forceForm.new_password
      ? ''
      : t('settings.passwordMismatch')
  return !forceErrors.old && !forceErrors.newPwd && !forceErrors.confirm
}

async function handleForceChange() {
  if (!validateForce()) return
  forceSubmitting.value = true
  try {
    await changePassword(forceForm.old_password, forceForm.new_password)
    toast.success(t('common.success'))
    showForceChange.value = false
    router.push('/dashboard')
  } catch {
    toast.error(t('common.failed'))
  } finally {
    forceSubmitting.value = false
  }
}

onMounted(preloadBackground)
</script>

<style scoped>
.login {
  position: relative;
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--fp-space-4);
  background: var(--fp-bg-app);
  overflow: hidden;
}
.login-bg {
  position: absolute;
  inset: 0;
  background: var(--fp-login-bg);
  background-size: cover;
  background-position: center;
  background-repeat: no-repeat;
  pointer-events: none;
}
.login-bg::after {
  content: '';
  position: absolute;
  inset: 0;
  background: rgb(0 0 0 / 0.35);
}
.login-glow {
  position: absolute;
  inset: 0;
  background:
    radial-gradient(700px 420px at 18% 12%, var(--fp-brand-soft), transparent 65%),
    radial-gradient(520px 360px at 85% 88%, color-mix(in srgb, var(--fp-brand-soft) 55%, transparent), transparent 60%);
  pointer-events: none;
}

.login-container {
  position: relative;
  z-index: 1;
  display: grid;
  grid-template-columns: 1.1fr 1fr;
  width: 880px;
  max-width: 100%;
  min-height: 460px;
  border-radius: var(--fp-radius-lg);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
  box-shadow: 0 24px 64px -24px rgb(0 0 0 / 0.3);
  overflow: hidden;
}

/* 左侧品牌区 */
.login-brand {
  background:
    radial-gradient(420px 300px at 30% 20%, var(--fp-brand-soft), transparent 70%),
    var(--fp-bg-sidebar);
  color: oklch(0.95 0 0);
  display: flex;
  align-items: center;
}
.login-brand__inner {
  padding: var(--fp-space-8);
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
}
.logo-mark {
  display: inline-flex;
  filter: drop-shadow(0 4px 20px var(--fp-brand-soft));
}
.login-brand__name {
  font-size: 26px;
  font-weight: 700;
  letter-spacing: 0.02em;
}
.login-brand__slogan {
  font-size: 13.5px;
  color: oklch(0.8 0.02 250);
  max-width: 280px;
}
.login-brand__features {
  list-style: none;
  margin-top: var(--fp-space-4);
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
}
.login-brand__features li {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  font-size: 13px;
  color: oklch(0.85 0.02 250);
}
.login-brand__features li i {
  color: var(--fp-brand);
  font-size: 14px;
}

/* 右侧表单 */
.login-form-panel {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--fp-space-8) var(--fp-space-6);
}
.login-form {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
  width: 100%;
  max-width: 320px;
}
.login-form__title {
  font-size: 20px;
  font-weight: 700;
  color: var(--fp-text-primary);
}
.login-form__sub {
  font-size: 13px;
  color: var(--fp-text-secondary);
  margin-bottom: var(--fp-space-2);
}
.login-btn {
  width: 100%;
  margin-top: var(--fp-space-2);
}

.force-alert {
  display: flex;
  align-items: flex-start;
  gap: var(--fp-space-2);
  padding: var(--fp-space-3);
  border-radius: var(--fp-radius-sm);
  background: var(--fp-warning-soft);
  color: var(--fp-warning);
  font-size: 13px;
  margin-bottom: var(--fp-space-4);
}
.force-form {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
}

@media (max-width: 640px) {
  .login-container {
    grid-template-columns: 1fr;
    max-width: 420px;
  }
  .login-brand {
    display: none;
  }
}
</style>
