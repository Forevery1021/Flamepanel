<template>
  <div class="panel security-panel">
    <div v-if="loading" class="security-loading">
      <FpSkeleton height="20px" />
      <FpSkeleton height="20px" />
      <FpSkeleton height="20px" />
    </div>
    <template v-else>
      <!-- 验签策略 -->
      <div class="security-block">
        <div class="security-block-title">
          <i class="oi oi-shield" />
          <span>{{ t('appStore.signaturePolicy') }}</span>
        </div>
        <p class="security-hint">{{ t('appStore.signaturePolicyHint') }}</p>
        <FpSelect
          v-model="signaturePolicy"
          :options="policyOptions"
          option-label="label"
          option-value="value"
          class="security-select"
        />
        <div
          class="security-alert-inline"
          :class="signaturePolicy === 'required' ? 'is-info' : 'is-warning'"
        >
          <i class="oi oi-info-circle" />
          <span>
            {{
              signaturePolicy === 'required'
                ? t('appStore.policyRequiredHint')
                : t('appStore.policyOptionalHint')
            }}
          </span>
        </div>
        <div
          v-if="config && config.unsigned_source_count > 0"
          class="security-alert-inline is-warning"
        >
          <i class="oi oi-exclamation-triangle" />
          <span>
            {{ t('appStore.unsignedSourceHint', { count: config.unsigned_source_count }) }}
          </span>
        </div>
      </div>

      <FpDivider />

      <!-- 镜像仓库白名单 -->
      <div class="security-block">
        <div class="security-block-title">
          <i class="oi oi-box" />
          <span>{{ t('appStore.registryWhitelist') }}</span>
        </div>
        <p class="security-hint">{{ t('appStore.registryWhitelistHint') }}</p>
        <div class="registry-add-row">
          <FpInput
            v-model="newRegistry"
            :placeholder="t('appStore.registryPlaceholder')"
            class="registry-input"
            @keyup.enter="addRegistry"
          />
          <FpButton variant="ghost" icon="oi oi-plus" @click="addRegistry">
            {{ t('common.add') }}
          </FpButton>
        </div>
        <div v-if="registryWhitelist.length" class="registry-chips">
          <FpChip
            v-for="reg in registryWhitelist"
            :key="reg"
            :label="reg"
            removable
            @remove="removeRegistry(reg)"
          />
        </div>
        <div v-else class="security-empty">{{ t('appStore.registryEmpty') }}</div>
        <div class="security-sub-title">{{ t('appStore.builtinRegistries') }}</div>
        <div class="registry-chips">
          <FpChip v-for="reg in config?.builtin_registries ?? []" :key="reg" :label="reg" />
        </div>
      </div>

      <div class="settings-actions security-actions">
        <FpButton variant="primary" :loading="saving" class="save-btn" @click="save">
          {{ t('common.save') }}
        </FpButton>
        <FpButton variant="ghost" @click="reload">{{ t('common.reset') }}</FpButton>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFpToast } from '@/components/ui/FpToast'
import type { StoreSecurityConfig } from '@/api/appStore'
import FpSkeleton from '@/components/ui/FpSkeleton.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpDivider from '@/components/ui/FpDivider.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpChip from '@/components/ui/FpChip.vue'

/**
 * 商店安全设置页签（Doc/23 §3.3 T2-4 视图拆分）。
 *
 * **编辑态归本组件**：验签策略与白名单是「本地暂存 → 整体提交」的会话
 * （`registry_whitelist` 后端为全量替换语义），故 `signaturePolicy` /
 * `registryWhitelist` 用本地 ref；服务端状态经 `config` prop 注入，
 * 变更时 `watch` 同步为编辑初值（含保存成功后的回写与「重置」）。
 *
 * 保存与失败文案由父级 `useAppStoreSecurity().save()` 承担，本组件经
 * `emit('save', payload)` 上抛载荷后按返回值决定是否清理本地态。
 */
const props = defineProps<{
  loading: boolean
  saving: boolean
  config: StoreSecurityConfig | null
}>()

/** 验签策略选项（文案属展示层，不进数据层） */
const policyOptions = computed(() => [
  { label: t('appStore.policyOptional'), value: 'optional' },
  { label: t('appStore.policyRequired'), value: 'required' },
])

const emit = defineEmits<{
  save: [payload: { signature_policy: string; registry_whitelist: string[] }]
  reload: []
}>()

const { t } = useI18n()
const toast = useFpToast()

const signaturePolicy = ref('optional')
const registryWhitelist = ref<string[]>([])
const newRegistry = ref('')

/** 服务端状态 → 编辑初值（首次加载、保存成功回写、显式重置三条路径共用） */
watch(
  () => props.config,
  (cfg) => {
    if (!cfg) return
    signaturePolicy.value = cfg.signature_policy
    registryWhitelist.value = [...cfg.registry_whitelist]
    newRegistry.value = ''
  },
  { immediate: true },
)

function reload() {
  newRegistry.value = ''
  emit('reload')
}

function addRegistry() {
  const value = newRegistry.value.trim().toLowerCase()
  if (!value) return
  if (registryWhitelist.value.includes(value)) {
    toast.warning(t('appStore.registryDuplicate'))
    return
  }
  registryWhitelist.value.push(value)
  newRegistry.value = ''
}

function removeRegistry(reg: string) {
  registryWhitelist.value = registryWhitelist.value.filter((r) => r !== reg)
}

function save() {
  emit('save', {
    signature_policy: signaturePolicy.value,
    registry_whitelist: [...registryWhitelist.value],
  })
}
</script>

<style scoped>
.security-panel {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
}
.security-loading {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
}
.security-block {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-2);
}
.security-block-title {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
  font-size: 14px;
  font-weight: 600;
  color: var(--fp-text-primary);
}
.security-block-title i {
  color: var(--fp-brand);
}
.security-hint {
  margin: 0;
  font-size: 12px;
  color: var(--fp-text-secondary);
  line-height: 1.6;
}
.security-select {
  max-width: 320px;
}
.security-alert-inline {
  display: flex;
  gap: var(--fp-space-2);
  align-items: flex-start;
  padding: var(--fp-space-2) var(--fp-space-3);
  border-radius: var(--fp-radius-md);
  font-size: 12px;
  line-height: 1.6;
}
.security-alert-inline.is-warning {
  background: var(--fp-warning-soft);
  color: var(--fp-text-primary);
  border: 1px solid var(--fp-warning);
}
.security-alert-inline.is-info {
  background: var(--fp-info-soft);
  color: var(--fp-text-primary);
  border: 1px solid var(--fp-info);
}
.security-alert-inline.is-warning i {
  color: var(--fp-warning);
  margin-top: 2px;
}
.security-alert-inline.is-info i {
  color: var(--fp-info);
  margin-top: 2px;
}
.registry-add-row {
  display: flex;
  gap: var(--fp-space-2);
  align-items: flex-start;
  max-width: 520px;
}
.registry-input {
  flex: 1;
}
.registry-chips {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fp-space-2);
}
.security-sub-title {
  margin-top: var(--fp-space-2);
  font-size: 12px;
  font-weight: 600;
  color: var(--fp-text-secondary);
}
.security-empty {
  font-size: 12px;
  color: var(--fp-text-muted);
}
.security-actions {
  display: flex;
  gap: var(--fp-space-2);
}
</style>
