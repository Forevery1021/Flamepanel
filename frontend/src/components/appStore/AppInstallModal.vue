<template>
  <FpModal
    v-model="visible"
    :header="t('appStore.installTitle') + ' - ' + (meta?.name || '')"
    size="lg"
  >
    <div v-if="securityWarning" class="security-alert">
      <i class="oi oi-exclamation-triangle" />
      <div>
        <div class="security-title">{{ t('appStore.securityWarning') }}</div>
        <div>{{ securityWarning }}</div>
      </div>
    </div>
    <!-- Doc/20 §4.3：生命周期钩子脚本确认 -->
    <div v-if="hookScripts.length" class="security-alert">
      <i class="oi oi-exclamation-triangle" />
      <div>
        <div class="security-title">{{ t('appStore.securityWarning') }}</div>
        <div>{{ t('appStore.hooksWarning', { scripts: hookScripts.join(', ') }) }}</div>
      </div>
    </div>
    <div class="modal-form">
      <FpSelect
        v-model="form.version"
        :label="t('appStore.fldVersion')"
        :options="versionOptions"
        option-label="label"
        option-value="value"
      />
      <FpSelect
        v-model="form.mode"
        :label="t('appStore.fldMode')"
        :options="modeOptions"
        option-label="label"
        option-value="value"
      />
      <FpInput
        v-model="form.name"
        :label="t('appStore.fldName')"
        :placeholder="t('appStore.fldNamePlaceholder')"
      />
      <div class="field-col">
        <label class="field-label">{{ t('appStore.fldPort') }}</label>
        <FpNumber v-model="form.port" :min="1" :max="65535" class="w-full" />
      </div>
      <FpInput v-model="form.container_name" :label="t('appStore.fldContainer')" />
      <template v-if="versionInfo">
        <FpDivider>{{ t('appStore.parameters') }}</FpDivider>
        <div v-for="f in versionInfo.form_fields" :key="f.env_key" class="modal-form">
          <FpSelect
            v-if="f.field_type === 'select'"
            v-model="form.values[f.env_key]"
            :label="f.label_zh"
            :options="f.options"
            option-label="label"
            option-value="value"
          />
          <FpSwitch
            v-else-if="f.field_type === 'switch'"
            v-model="switchValues[f.env_key]"
            :label="f.label_zh"
          />
          <div v-else-if="f.field_type === 'number' || f.field_type === 'port'" class="field-col">
            <label class="field-label">{{ f.label_zh }}</label>
            <FpNumber
              v-model="numberValues[f.env_key]"
              :min="f.min ?? undefined"
              :max="f.max ?? undefined"
              class="w-full"
            />
          </div>
          <FpInput
            v-else
            v-model="form.values[f.env_key]"
            :label="f.label_zh"
            :type="f.field_type === 'password' ? 'password' : 'text'"
            :placeholder="f.description || ''"
          />
        </div>
      </template>
    </div>
    <template #footer>
      <div class="install-footer">
        <FpCheckbox v-model="form.confirm_risky">
          {{ t('appStore.confirmRisky') }}
        </FpCheckbox>
        <FpCheckbox v-if="hookScripts.length" v-model="form.acknowledge_scripts">
          {{ t('appStore.hooksAck') }}
        </FpCheckbox>
        <div class="footer-btns">
          <FpButton variant="ghost" @click="visible = false">
            {{ t('common.cancel') }}
          </FpButton>
          <FpButton variant="primary" :loading="loading" @click="onSubmit">{{
            t('appStore.install')
          }}</FpButton>
        </div>
      </div>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useAppStoreInstall } from '@/composables/useAppStoreInstall'
import type { AppMetadata } from '@/api/appStore'
import FpModal from '@/components/ui/FpModal.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpCheckbox from '@/components/ui/FpCheckbox.vue'
import FpDivider from '@/components/ui/FpDivider.vue'
import FpNumber from '@/components/ui/FpNumber.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'

/**
 * 应用安装弹窗（Doc/23 §3.3 T2-4 视图拆分）。
 *
 * 状态机全部落在 `useAppStoreInstall`：版本/参数加载、风险提示（privileged、
 * docker.sock、钩子脚本）、提交。本组件只负责模板装配与「安装成功」事件上抛——
 * 视图层据此刷新已安装列表（拆分的唯一跨模块耦合点）。
 *
 * `open` 经 `defineExpose` 暴露：视图层在点击卡片后调用，避免把「待安装包」
 * 当作响应式 prop 层层传递。
 */
const install = useAppStoreInstall()
const {
  visible,
  loading,
  meta,
  versionInfo,
  securityWarning,
  form,
  switchValues,
  numberValues,
  versionOptions,
  modeOptions,
  hookScripts,
  submit,
} = install

const emit = defineEmits<{ installed: [] }>()

const { t } = useI18n()

async function onSubmit() {
  if (await submit()) emit('installed')
}

/** 供父组件调用：打开弹窗并加载该包的版本信息 */
function open(pkg: AppMetadata) {
  return install.open(pkg)
}

defineExpose({ open })
</script>

<style scoped>
.security-alert {
  display: flex;
  gap: var(--fp-space-2);
  align-items: flex-start;
  padding: var(--fp-space-3);
  border-radius: var(--fp-radius-md);
  background: var(--fp-warning-soft);
  border: 1px solid var(--fp-warning);
  font-size: 13px;
  color: var(--fp-text-primary);
  margin-bottom: var(--fp-space-4);
}
.security-alert i {
  color: var(--fp-warning);
  margin-top: 2px;
}
.security-title {
  font-weight: 600;
  color: var(--fp-warning);
  margin-bottom: 2px;
}
.install-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  width: 100%;
  gap: var(--fp-space-3);
}
.footer-btns {
  display: flex;
  gap: var(--fp-space-2);
}
</style>
