<template>
  <div class="settings-group fp-surface tf-panel">
    <h3 class="settings-group__title">
      {{ t('twoFactor.title') }}
      <FpTag
        v-if="status"
        :severity="status.enabled ? 'success' : 'neutral'"
        :value="status.enabled ? t('twoFactor.statusEnabled') : t('twoFactor.statusDisabled')"
      />
    </h3>
    <p class="tf-desc">{{ t('twoFactor.desc') }}</p>
    <FpInlineMessage v-if="status?.required" severity="info" class="tf-msg">
      {{ t('twoFactor.requiredHint') }}
    </FpInlineMessage>
    <p v-if="status?.enabled" class="tf-meta">
      <span v-if="status.enabled_at">{{
        t('twoFactor.enabledAt', { time: formatTime(status.enabled_at) })
      }}</span>
      <span>{{ t('twoFactor.recoveryRemaining', { n: status.recovery_codes_remaining }) }}</span>
    </p>
    <div class="settings-group__foot tf-actions">
      <template v-if="status?.enabled">
        <FpButton variant="secondary" outlined @click="open('setup')">
          {{ t('twoFactor.rebind') }}
        </FpButton>
        <FpButton variant="secondary" outlined @click="open('regen')">
          {{ t('twoFactor.regenerate') }}
        </FpButton>
        <FpButton v-if="!status.required" variant="danger" outlined @click="open('disable')">
          {{ t('twoFactor.disable') }}
        </FpButton>
      </template>
      <FpButton v-else :loading="loading" :disabled="!status" @click="open('setup')">
        {{ t('twoFactor.enable') }}
      </FpButton>
    </div>

    <FpModal v-model="visible" :header="modalTitle" :closable="step !== 'codes'" size="md">
      <!-- 步骤 1：确认密码（setup） -->
      <form v-if="step === 'password'" class="tf-form" @submit.prevent="submitPassword">
        <p class="tf-hint">{{ t('twoFactor.passwordHint') }}</p>
        <FpInput
          v-model="form.password"
          :label="t('twoFactor.password')"
          type="password"
          autocomplete="current-password"
          :error="errors.password"
          @update:model-value="errors.password = ''"
        />
      </form>

      <!-- 步骤 2：添加到认证器 + 输入验证码 -->
      <form v-else-if="step === 'scan' && setup" class="tf-form" @submit.prevent="submitEnable">
        <p class="tf-hint">{{ t('twoFactor.scanStep') }}</p>
        <a class="tf-link" :href="setup.otpauth_uri">
          <i class="oi oi-external-link" /> {{ t('twoFactor.openLink') }}
        </a>
        <div class="tf-secret">
          <span class="tf-secret__label">{{ t('twoFactor.secretKey') }}</span>
          <code class="tf-secret__value mono">{{ groupedSecret }}</code>
          <CopyButton :text="setup.secret" />
        </div>
        <FpInput
          v-model="form.code"
          :label="t('twoFactor.code')"
          :placeholder="t('twoFactor.codePlaceholder')"
          inputmode="numeric"
          autocomplete="one-time-code"
          maxlength="7"
          :error="errors.code"
          @update:model-value="errors.code = ''"
        />
        <p class="tf-hint">{{ t('twoFactor.codeHint') }}</p>
      </form>

      <!-- 恢复码展示（仅此一次） -->
      <div v-else-if="step === 'codes'" class="tf-form">
        <FpInlineMessage severity="warn">{{ t('twoFactor.recoveryHint') }}</FpInlineMessage>
        <ul class="tf-codes mono">
          <li v-for="c in recoveryCodes" :key="c">{{ c }}</li>
        </ul>
        <div class="tf-codes__actions">
          <CopyButton :text="recoveryCodes.join('\n')" />
          <FpButton variant="ghost" icon="oi oi-download" @click="downloadCodes">
            {{ t('twoFactor.download') }}
          </FpButton>
        </div>
      </div>

      <!-- 停用 / 重新生成：密码 + 验证码 -->
      <form v-else-if="step === 'confirm'" class="tf-form" @submit.prevent="submitConfirm">
        <p class="tf-hint">
          {{ mode === 'disable' ? t('twoFactor.disableHint') : t('twoFactor.regenHint') }}
        </p>
        <FpInput
          v-model="form.password"
          :label="t('twoFactor.password')"
          type="password"
          autocomplete="current-password"
          :error="errors.password"
          @update:model-value="errors.password = ''"
        />
        <FpInput
          v-model="form.code"
          :label="mode === 'disable' ? t('twoFactor.codeOrRecovery') : t('twoFactor.code')"
          autocomplete="one-time-code"
          :error="errors.code"
          @update:model-value="errors.code = ''"
        />
      </form>

      <template #footer>
        <FpButton v-if="step === 'codes'" @click="finishCodes">{{ t('twoFactor.saved') }}</FpButton>
        <template v-else>
          <FpButton variant="ghost" @click="visible = false">{{ t('common.cancel') }}</FpButton>
          <FpButton
            :variant="mode === 'disable' ? 'danger' : 'primary'"
            :loading="submitting"
            :disabled="!canSubmit"
            @click="submitCurrent"
          >
            {{ primaryLabel }}
          </FpButton>
        </template>
      </template>
    </FpModal>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  beginTwoFactorSetup,
  disableTwoFactor,
  enableTwoFactor,
  fetchTwoFactorStatus,
  regenerateRecoveryCodes,
} from '@/api/auth'
import type { TwoFactorSetup, TwoFactorStatusResponse } from '@/api/generated'
import FpButton from '@/components/ui/FpButton.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpInlineMessage from '@/components/ui/FpInlineMessage.vue'
import CopyButton from '@/components/ui/CopyButton.vue'
import { useFpToast } from '@/components/ui/FpToast'

/**
 * 两步验证（TOTP）自助管理面板：启用 / 更换认证器 / 重新生成恢复码 / 停用。
 *
 * 独立组件（不继续膨胀 SettingsView）：设置页「安全」分区与强制绑定页 `/two-factor-setup` 共用。
 * 流程：确认密码 → 展示 otpauth 链接与密钥 → 输入验证码启用 → 一次性展示恢复码。
 * 不引入二维码依赖：手机上点链接可直接唤起认证器，桌面端手动输入密钥。
 */
const emit = defineEmits<{ (e: 'enabled'): void; (e: 'changed'): void }>()

type Mode = 'setup' | 'disable' | 'regen'
type Step = 'password' | 'scan' | 'codes' | 'confirm'

const { t } = useI18n()
const toast = useFpToast()

const status = ref<TwoFactorStatusResponse | null>(null)
const loading = ref(false)
const visible = ref(false)
const mode = ref<Mode>('setup')
const step = ref<Step>('password')
const submitting = ref(false)
const setup = ref<TwoFactorSetup | null>(null)
const recoveryCodes = ref<string[]>([])
const form = reactive({ password: '', code: '' })
const errors = reactive({ password: '', code: '' })

const modalTitle = computed(() => {
  if (step.value === 'codes') return t('twoFactor.recoveryTitle')
  if (mode.value === 'disable') return t('twoFactor.disableTitle')
  if (mode.value === 'regen') return t('twoFactor.regenTitle')
  return status.value?.enabled ? t('twoFactor.rebind') : t('twoFactor.enable')
})

const primaryLabel = computed(() => {
  if (step.value === 'password') return t('twoFactor.next')
  if (step.value === 'scan') return t('twoFactor.confirmEnable')
  return mode.value === 'disable' ? t('twoFactor.disable') : t('twoFactor.regenerate')
})

const canSubmit = computed(() => {
  if (step.value === 'password') return !!form.password
  if (step.value === 'scan') return /^\d{6}$/.test(form.code.replace(/\s/g, ''))
  return !!form.password && !!form.code.trim()
})

/** 密钥每 4 位一组，便于手动输入 */
const groupedSecret = computed(() => setup.value?.secret.replace(/(.{4})/g, '$1 ').trim() ?? '')

function formatTime(iso: string) {
  const dt = new Date(iso)
  return Number.isNaN(dt.getTime()) ? iso : dt.toLocaleString()
}

async function load() {
  loading.value = true
  try {
    status.value = (await fetchTwoFactorStatus()).data
  } catch (e) {
    toast.error(e, t('common.failed'))
  } finally {
    loading.value = false
  }
}

function open(m: Mode) {
  mode.value = m
  step.value = m === 'setup' ? 'password' : 'confirm'
  form.password = ''
  form.code = ''
  errors.password = ''
  errors.code = ''
  setup.value = null
  visible.value = true
}

/** 后端对密码 / 验证码错误返回 400，消息区分二者 */
function applyError(e: unknown) {
  const err = e as { status?: number; message?: string }
  if (err.status === 400 && /password/i.test(err.message ?? '')) {
    errors.password = t('login.error')
  } else if (err.status === 400) {
    errors.code = t('twoFactor.loginInvalid')
  } else {
    toast.error(e, t('common.failed'))
  }
}

async function submitPassword() {
  if (!canSubmit.value) return
  submitting.value = true
  try {
    setup.value = (await beginTwoFactorSetup(form.password)).data
    form.password = ''
    step.value = 'scan'
  } catch (e) {
    applyError(e)
  } finally {
    submitting.value = false
  }
}

async function submitEnable() {
  if (!canSubmit.value) return
  submitting.value = true
  try {
    recoveryCodes.value = (await enableTwoFactor(form.code.replace(/\s/g, ''))).data.recovery_codes
    step.value = 'codes'
    toast.success(t('twoFactor.enabledOk'))
    emit('enabled')
  } catch (e) {
    applyError(e)
  } finally {
    submitting.value = false
  }
}

async function submitConfirm() {
  if (!canSubmit.value) return
  submitting.value = true
  try {
    if (mode.value === 'disable') {
      await disableTwoFactor(form.password, form.code.trim())
      toast.success(t('twoFactor.disabledOk'))
      visible.value = false
      emit('changed')
      await load()
    } else {
      recoveryCodes.value = (
        await regenerateRecoveryCodes(form.password, form.code.replace(/\s/g, ''))
      ).data.recovery_codes
      step.value = 'codes'
    }
  } catch (e) {
    applyError(e)
  } finally {
    submitting.value = false
  }
}

function submitCurrent() {
  if (step.value === 'password') return submitPassword()
  if (step.value === 'scan') return submitEnable()
  return submitConfirm()
}

function downloadCodes() {
  const blob = new Blob([recoveryCodes.value.join('\n') + '\n'], { type: 'text/plain' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = 'flamepanel-recovery-codes.txt'
  a.click()
  URL.revokeObjectURL(url)
}

async function finishCodes() {
  recoveryCodes.value = []
  visible.value = false
  emit('changed')
  await load()
}

onMounted(load)

defineExpose({ reload: load })
</script>

<style scoped>
.settings-group__title {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
}
.tf-desc,
.tf-hint,
.tf-meta {
  margin: 0 0 var(--fp-space-3);
  color: var(--fp-text-secondary);
  font-size: var(--fp-font-size-sm);
  line-height: 1.6;
}
.tf-meta {
  display: flex;
  gap: var(--fp-space-4);
  flex-wrap: wrap;
}
.tf-msg {
  margin-bottom: var(--fp-space-3);
}
.tf-actions {
  display: flex;
  gap: var(--fp-space-2);
  flex-wrap: wrap;
}
.tf-form {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
}
.tf-link {
  color: var(--fp-brand);
  font-weight: 500;
  text-decoration: none;
}
.tf-secret {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  padding: var(--fp-space-2) var(--fp-space-3);
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-subtle);
}
.tf-secret__label {
  color: var(--fp-text-secondary);
  font-size: var(--fp-font-size-sm);
  white-space: nowrap;
}
.tf-secret__value {
  flex: 1;
  word-break: break-all;
  letter-spacing: 0.04em;
}
.tf-codes {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--fp-space-2) var(--fp-space-4);
  margin: 0;
  padding: var(--fp-space-3);
  list-style: none;
  border: 1px dashed var(--fp-border);
  border-radius: var(--fp-radius-md);
}
.tf-codes__actions {
  display: flex;
  gap: var(--fp-space-2);
  align-items: center;
}
</style>
