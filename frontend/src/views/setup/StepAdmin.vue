<template>
  <div class="step">
    <h2 class="step__title">{{ t('setup.adminTitle') }}</h2>
    <p class="step__sub">{{ t('setup.adminSub') }}</p>

    <div class="step-form">
      <FpInput
        v-model="username"
        :label="t('setup.adminUsername')"
        :error="errors.username"
        autocomplete="username"
        @update:model-value="emit('update', { username: $event })"
      />
      <FpInput
        v-model="password"
        :label="t('setup.adminPassword')"
        type="password"
        toggle-mask
        :error="errors.password"
        autocomplete="new-password"
        @update:model-value="emit('update', { password: $event })"
      />
      <!-- 密码强度：四段条 + 文案，随输入实时变化（纯启发式，见 utils/password.ts） -->
      <div
        v-if="password"
        class="pw-meter"
        :class="`is-${score}`"
        role="meter"
        :aria-valuenow="score"
        aria-valuemin="0"
        aria-valuemax="4"
        :aria-label="t('setup.strength.label')"
      >
        <span class="pw-meter__bars" aria-hidden="true">
          <i v-for="n in 4" :key="n" class="pw-meter__bar" :class="{ 'is-on': n <= score }" />
        </span>
        <span class="pw-meter__text">{{ t(`setup.strength.${scoreKey}`) }}</span>
      </div>
      <FpInput
        v-model="confirm"
        :label="t('setup.adminConfirm')"
        type="password"
        toggle-mask
        :error="errors.confirm"
        autocomplete="new-password"
        @update:model-value="emit('update', { confirm: $event })"
      />
    </div>

    <p class="step-hint">{{ t('setup.adminHint') }}</p>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import FpInput from '@/components/ui/FpInput.vue'
import { passwordScore, PASSWORD_SCORE_KEYS } from '@/utils/password'

const props = defineProps<{
  username: string
  password: string
  confirm: string
  errors: { username: string; password: string; confirm: string }
}>()

const emit = defineEmits<{
  update: [patch: Partial<{ username: string; password: string; confirm: string }>]
}>()

const { t } = useI18n()
// 本地 v-model 与父级同步（输入即时校验）
const username = computed({
  get: () => props.username,
  set: (v) => emit('update', { username: v }),
})
const password = computed({
  get: () => props.password,
  set: (v) => emit('update', { password: v }),
})
const confirm = computed({
  get: () => props.confirm,
  set: (v) => emit('update', { confirm: v }),
})
const score = computed(() => passwordScore(props.password))
const scoreKey = computed(() => PASSWORD_SCORE_KEYS[score.value])
</script>

<style scoped>
.step {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
}
.step__title {
  font-size: 20px;
  font-weight: 700;
  color: var(--fp-text-primary);
}
.step__sub {
  font-size: 13px;
  color: var(--fp-text-secondary);
}
.step-form {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
  max-width: 320px;
  margin-top: var(--fp-space-2);
}
.step-hint {
  font-size: 12.5px;
  color: var(--fp-text-secondary);
}

/* ── 密码强度 ── */
.pw-meter {
  display: flex;
  align-items: center;
  gap: var(--fp-space-3);
  margin-top: calc(-1 * var(--fp-space-2));
}
.pw-meter__bars {
  display: flex;
  gap: 4px;
  flex: 1;
}
.pw-meter__bar {
  display: block;
  height: 4px;
  flex: 1;
  border-radius: 999px;
  background: var(--fp-border);
  transition: background-color var(--fp-transition-fast);
}
.pw-meter__text {
  font-size: 12px;
  min-width: 3.5em;
  text-align: right;
  color: var(--fp-text-muted);
  font-variant-numeric: tabular-nums;
}
.pw-meter.is-0 .pw-meter__bar.is-on,
.pw-meter.is-1 .pw-meter__bar.is-on {
  background: var(--fp-danger);
}
.pw-meter.is-2 .pw-meter__bar.is-on {
  background: var(--fp-warning);
}
.pw-meter.is-3 .pw-meter__bar.is-on {
  background: var(--fp-success);
}
.pw-meter.is-4 .pw-meter__bar.is-on {
  background: var(--fp-brand);
}
.pw-meter.is-3 .pw-meter__text,
.pw-meter.is-4 .pw-meter__text {
  color: var(--fp-text-secondary);
}
</style>
