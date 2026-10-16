<template>
  <FpModal
    :model-value="open"
    :header="mode === 'create' ? t('firewall.add') : t('firewall.edit')"
    size="md"
    @update:model-value="emit('update:open', $event)"
  >
    <form class="modal-form" novalidate @submit.prevent="onSubmit">
      <FpInput
        v-model="form.name"
        :label="t('firewall.name')"
        :placeholder="t('firewall.namePlaceholder')"
        :error="msg('name')"
        class="field-name"
      />
      <FpInput
        v-model="form.description"
        :label="t('firewall.description')"
        :placeholder="t('firewall.descPlaceholder')"
        :error="msg('description')"
      />
      <div class="grid-2">
        <FpSelect
          v-model="form.protocol"
          :label="t('firewall.protocol')"
          :options="protocolOptions"
          option-label="label"
          option-value="value"
          class="field-protocol"
        />
        <FpSelect
          v-model="form.direction"
          :label="t('firewall.direction')"
          :options="directionOptions"
          option-label="label"
          option-value="value"
          class="field-direction"
        />
      </div>
      <div class="field-with-hint">
        <FpInput
          v-model="form.port"
          :label="t('firewall.port')"
          :placeholder="portEnabled ? t('firewall.portPlaceholder') : t('firewall.any')"
          :disabled="!portEnabled"
          :error="msg('port')"
          class="field-port"
        />
        <small v-if="!msg('port')" class="field-hint">
          {{ portEnabled ? t('firewall.hints.port') : t('firewall.hints.portDisabled') }}
        </small>
      </div>
      <div class="field-with-hint">
        <FpInput
          v-model="form.source"
          :label="form.direction === 'out' ? t('firewall.target') : t('firewall.source')"
          placeholder="any"
          :error="msg('source')"
          class="field-source"
        />
        <small v-if="!msg('source')" class="field-hint">
          <template v-if="normalizedSource">
            {{ t('firewall.hints.normalized', { value: normalizedSource }) }}
          </template>
          <template v-else>
            {{ form.direction === 'out' ? t('firewall.hints.target') : t('firewall.hints.source') }}
          </template>
        </small>
      </div>
      <div class="grid-2">
        <FpSelect
          v-model="form.action"
          :label="t('firewall.action')"
          :options="actionOptions"
          option-label="label"
          option-value="value"
        />
        <FpNumber
          v-model="form.priority"
          :label="t('firewall.priority')"
          :min="PRIORITY_MIN"
          :max="PRIORITY_MAX"
          :error="msg('priority')"
        />
      </div>
      <FpInlineMessage v-if="limitation" severity="warn" class="backend-limit">
        {{ t(limitation) }}
      </FpInlineMessage>
      <!-- 让回车提交生效 -->
      <button type="submit" hidden />
    </form>
    <template #footer>
      <FpButton variant="ghost" @click="emit('update:open', false)">
        {{ t('common.cancel') }}
      </FpButton>
      <FpButton variant="primary" :loading="saving" class="rule-submit" @click="onSubmit">
        {{ mode === 'create' ? t('common.confirm') : t('common.save') }}
      </FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpNumber from '@/components/ui/FpNumber.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpInlineMessage from '@/components/ui/FpInlineMessage.vue'
import type { FirewallRule } from '@/types'
import {
  PRIORITY_MAX,
  PRIORITY_MIN,
  backendLimitation,
  emptyRuleForm,
  formFromRule,
  normalizeAddress,
  portsAllowed,
  toRulePayload,
  validateRuleForm,
  type FirewallRuleForm,
} from '@/utils/firewallRule'

/**
 * S-13 防火墙规则新建 / 编辑弹窗（两种模式共用一张表单）。
 *
 * 校验与后端 `domain/firewall.rs` 同口径：首次提交后逐项实时提示；ICMP / 任意协议禁用端口；
 * 出站规则把地址标注为「目标 IP」；CIDR 主机位会提示规范化后的保存值；按当前后端提示不支持的组合。
 * 只负责收集并规范化请求体，真正的请求由页面处理。
 */
const props = defineProps<{
  open: boolean
  mode: 'create' | 'edit'
  rule?: FirewallRule | null
  /** 当前防火墙后端（`ufw` / `firewalld` / `iptables`），用于提示限制 */
  backend?: string | null
  saving?: boolean
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
  submit: [payload: Record<string, string | number>]
}>()

const { t } = useI18n()

const form = ref<FirewallRuleForm>(emptyRuleForm())
const submitted = ref(false)

watch(
  () => props.open,
  (open) => {
    if (!open) return
    form.value = props.mode === 'edit' && props.rule ? formFromRule(props.rule) : emptyRuleForm()
    submitted.value = false
  },
  { immediate: true },
)

const portEnabled = computed(() => portsAllowed(form.value.protocol))
// 切到 ICMP / 任意协议：清空端口（后端也会拒绝带端口的组合）
watch(portEnabled, (enabled) => {
  if (!enabled) form.value.port = ''
})

const errors = computed(() => validateRuleForm(form.value))

function msg(field: keyof FirewallRuleForm): string {
  if (!submitted.value) return ''
  const e = errors.value[field]
  return e ? t(e.key, e.params ?? {}) : ''
}

/** 规范化后与输入不同时才提示（如 `192.168.1.10/24` → `192.168.1.0/24`） */
const normalizedSource = computed(() => {
  const r = normalizeAddress(form.value.source)
  return r.ok && r.value && r.value !== form.value.source.trim() ? r.value : ''
})

const limitation = computed(() => backendLimitation(props.backend, form.value))

const protocolOptions = computed(() => [
  { label: 'TCP', value: 'tcp' },
  { label: 'UDP', value: 'udp' },
  { label: 'ICMP', value: 'icmp' },
  { label: t('firewall.any'), value: 'any' },
])
const actionOptions = computed(() => [
  { label: `${t('firewall.allow')} (ALLOW)`, value: 'allow' },
  { label: `${t('firewall.deny')} (DENY)`, value: 'deny' },
  { label: `${t('firewall.reject')} (REJECT)`, value: 'reject' },
])
const directionOptions = computed(() => [
  { label: t('firewall.in'), value: 'in' },
  { label: t('firewall.out'), value: 'out' },
])

function onSubmit() {
  submitted.value = true
  if (props.saving || Object.keys(errors.value).length) return
  emit('submit', toRulePayload(form.value))
}
</script>

<style scoped>
.modal-form {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
}
.grid-2 {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--fp-space-3);
}
.field-with-hint {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.field-hint {
  font-size: var(--fp-font-xs);
  color: var(--fp-text-muted);
  line-height: 1.4;
}
@media (max-width: 560px) {
  .grid-2 {
    grid-template-columns: 1fr;
  }
}
</style>
