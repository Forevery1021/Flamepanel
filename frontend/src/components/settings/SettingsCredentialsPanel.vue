<template>
  <div class="settings-group fp-surface">
    <h3 class="settings-group__title">{{ t('settings.changePassword') }}</h3>
    <form autocomplete="off" @submit.prevent="handleChangePassword">
      <SettingRow v-slot="{ id }" :label="t('settings.oldPassword')">
        <FpInput
          v-model="pwForm.old_password"
          :input-id="id"
          type="password"
          autocomplete="current-password"
        />
      </SettingRow>
      <SettingRow
        v-slot="{ id }"
        :label="t('settings.newPassword')"
        :description="t('settings.passwordRule')"
      >
        <FpInput
          v-model="pwForm.new_password"
          :input-id="id"
          type="password"
          autocomplete="new-password"
        />
      </SettingRow>
      <SettingRow v-slot="{ id }" :label="t('settings.confirmPassword')">
        <FpInput
          v-model="pwForm.confirm"
          :input-id="id"
          type="password"
          autocomplete="new-password"
        />
      </SettingRow>
      <div class="settings-group__foot">
        <FpButton type="submit" :loading="pwSubmitting" :disabled="!pwReady">
          {{ t('settings.updatePassword') }}
        </FpButton>
      </div>
    </form>
  </div>

  <TwoFactorPanel />

  <div class="settings-group fp-surface">
    <h3 class="settings-group__title">{{ t('settings.groupKeys') }}</h3>
    <SettingRow :label="t('settings.jwtSecret')" :description="t('settings.jwtSecretHint')">
      <FpButton variant="warning" outlined :loading="rotating" @click="handleRotateJwtSecret">
        {{ t('settings.rotate') }}
      </FpButton>
    </SettingRow>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'

import { changePassword, rotateJwtSecret } from '@/api/auth'
import { useAuthStore } from '@/stores/auth'
import FpButton from '@/components/ui/FpButton.vue'
import FpInput from '@/components/ui/FpInput.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import SettingRow from '@/components/settings/SettingRow.vue'
import TwoFactorPanel from '@/components/settings/TwoFactorPanel.vue'

/**
 * 面板设置 · 安全分区的「凭据」部分：改密、二次验证、JWT 密钥轮换。
 * 三者都是即时操作（不进入安全分区的批量保存/脏状态），因此从 SettingsView 拆出独立维护。
 */
const { t } = useI18n()
const router = useRouter()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()
const auth = useAuthStore()

const pwSubmitting = ref(false)
const rotating = ref(false)
const pwForm = reactive({ old_password: '', new_password: '', confirm: '' })
const pwReady = computed(() => !!pwForm.old_password && !!pwForm.new_password && !!pwForm.confirm)

async function handleChangePassword() {
  if (pwForm.confirm !== pwForm.new_password) {
    toast.warning(t('settings.passwordMismatch'))
    return
  }
  pwSubmitting.value = true
  try {
    await changePassword(pwForm.old_password, pwForm.new_password)
    toast.success(t('common.success'))
    Object.assign(pwForm, { old_password: '', new_password: '', confirm: '' })
  } catch (e: unknown) {
    toast.error(e, t('common.failed'))
  } finally {
    pwSubmitting.value = false
  }
}

function handleRotateJwtSecret() {
  confirmAction({
    header: t('settings.rotate'),
    message: t('settings.rotateConfirm'),
    acceptLabel: t('settings.rotate'),
    rejectLabel: t('common.cancel'),
    accept: async () => {
      rotating.value = true
      try {
        // CSPRNG 生成 48 字节（96 hex），满足服务端 ≥32 字节下限
        const bytes = new Uint8Array(48)
        crypto.getRandomValues(bytes)
        const secret = Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('')
        await rotateJwtSecret(secret)
        toast.success(t('settings.rotateDone'))
        auth.logout()
        router.push('/login').catch(() => {})
      } catch (e: unknown) {
        toast.error(e, t('common.failed'))
      } finally {
        rotating.value = false
      }
    },
  })
}
</script>
