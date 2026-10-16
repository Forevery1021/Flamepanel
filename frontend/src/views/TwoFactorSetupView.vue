<template>
  <div class="tfs">
    <div class="tfs-card fp-surface">
      <h1 class="tfs-title">
        <i class="oi oi-shield" />
        {{ t('twoFactor.setupPageTitle') }}
      </h1>
      <p class="tfs-hint">{{ t('twoFactor.setupPageHint') }}</p>
      <TwoFactorPanel @enabled="done = true" />
      <div class="tfs-foot">
        <FpButton variant="ghost" @click="signOut">{{ t('twoFactor.logout') }}</FpButton>
        <FpButton :disabled="!done" @click="router.push('/dashboard')">
          {{ t('twoFactor.continue') }}
        </FpButton>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import FpButton from '@/components/ui/FpButton.vue'
import TwoFactorPanel from '@/components/settings/TwoFactorPanel.vue'

/**
 * 强制两步验证绑定页：面板设置 `two_factor_enabled=true` 且当前用户未绑定时，
 * 登录后（或任一接口返回 `TWO_FACTOR_SETUP_REQUIRED`）进入此页；绑定完成才能进入面板。
 */
const { t } = useI18n()
const router = useRouter()
const auth = useAuthStore()
const done = ref(false)

function signOut() {
  auth.logout()
  router.push('/login')
}
</script>

<style scoped>
.tfs {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--fp-space-4);
  background: var(--fp-bg);
}
.tfs-card {
  width: min(640px, 100%);
  padding: var(--fp-space-6);
  border-radius: var(--fp-radius-lg);
}
.tfs-title {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  margin: 0 0 var(--fp-space-2);
  font-size: var(--fp-font-size-xl);
}
.tfs-hint {
  margin: 0 0 var(--fp-space-4);
  color: var(--fp-text-secondary);
}
.tfs-foot {
  display: flex;
  justify-content: space-between;
  margin-top: var(--fp-space-4);
}
</style>
