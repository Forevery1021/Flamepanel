<template>
  <div class="panel assistant-settings">
    <h3 class="section-title">{{ t('assistant.settingsTitle') }}</h3>
    <p class="muted">{{ t('assistant.settingsHint') }}</p>
    <FpInlineMessage v-if="!canEdit" severity="info">{{ t('assistant.readOnly') }}</FpInlineMessage>

    <form class="form" @submit.prevent="save">
      <FpSwitch v-model="form.enabled" :label="t('assistant.enabled')" :disabled="!canEdit" />
      <div class="grid">
        <FpSelect
          v-model="form.provider"
          :label="t('assistant.provider')"
          :options="providerOptions"
          option-label="label"
          option-value="value"
          :disabled="!canEdit"
        />
        <FpInput
          v-model="form.model"
          :label="t('assistant.model')"
          :placeholder="form.provider === 'ollama' ? 'qwen2.5:7b' : 'gpt-4o-mini'"
          :error="modelError"
          maxlength="128"
          :disabled="!canEdit"
        />
      </div>
      <FpInput
        v-model="form.baseUrl"
        :label="t('assistant.baseUrl')"
        :placeholder="defaultUrl"
        maxlength="512"
        :disabled="!canEdit"
      />
      <small class="muted">{{ t('assistant.baseUrlHint', { url: defaultUrl }) }}</small>
      <FpInput
        v-model="form.apiKey"
        type="password"
        autocomplete="new-password"
        :label="t('assistant.apiKey')"
        :placeholder="config?.has_api_key ? t('assistant.apiKeySet') : t('assistant.apiKeyNone')"
        maxlength="1024"
        :disabled="!canEdit"
      />
      <FpButton
        v-if="canEdit && config?.has_api_key"
        variant="link"
        size="small"
        class="clear-key"
        :label="t('assistant.clearKey')"
        @click="clearKey"
      />
      <div class="grid three">
        <FpNumber
          v-model="form.temperature"
          :label="t('assistant.temperature')"
          :min="0"
          :max="2"
          :step="0.1"
          :min-fraction-digits="1"
          :max-fraction-digits="2"
          :disabled="!canEdit"
        />
        <FpNumber
          v-model="form.maxSteps"
          :label="t('assistant.maxSteps')"
          :min="1"
          :max="12"
          :disabled="!canEdit"
        />
        <FpNumber
          v-model="form.timeoutSecs"
          :label="t('assistant.timeout')"
          :min="5"
          :max="300"
          :disabled="!canEdit"
        />
      </div>
    </form>

    <div v-if="canEdit" class="actions">
      <FpButton
        variant="secondary"
        icon="oi oi-link"
        :label="t('assistant.test')"
        :loading="testing"
        @click="test"
      />
      <FpButton icon="oi oi-check" :label="t('assistant.save')" :loading="saving" @click="save" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import FpButton from '@/components/ui/FpButton.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpNumber from '@/components/ui/FpNumber.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import FpInlineMessage from '@/components/ui/FpInlineMessage.vue'
import { useFpToast } from '@/components/ui/FpToast'
import {
  testAssistant,
  updateAssistantConfig,
  type AssistantConfigPatch,
  type AssistantConfigView,
} from '@/api/assistant'

/**
 * 助手设置（M-AI-3）：Provider / 模型 / 密钥。密钥只写不读——输入框留空表示保持不变。
 */
const props = defineProps<{ config: AssistantConfigView | null; canEdit: boolean }>()
const emit = defineEmits<{ saved: [config: AssistantConfigView] }>()

const { t } = useI18n()
const toast = useFpToast()

const DEFAULT_URLS = {
  openai: 'https://api.openai.com/v1',
  ollama: 'http://127.0.0.1:11434',
} as const
type Provider = keyof typeof DEFAULT_URLS

const form = reactive({
  enabled: false,
  provider: 'openai' as Provider,
  baseUrl: '',
  model: '',
  apiKey: '',
  temperature: 0.2 as number | null,
  maxSteps: 6 as number | null,
  timeoutSecs: 60 as number | null,
})
const touched = ref(false)
const saving = ref(false)
const testing = ref(false)

watch(
  () => props.config,
  (c) => {
    if (!c) return
    form.enabled = c.enabled
    form.provider = c.provider as Provider
    form.baseUrl = c.base_url
    form.model = c.model
    form.apiKey = ''
    form.temperature = c.temperature
    form.maxSteps = c.max_steps
    form.timeoutSecs = c.timeout_secs
  },
  { immediate: true },
)

const providerOptions = computed(() => [
  { value: 'openai', label: t('assistant.providers.openai') },
  { value: 'ollama', label: t('assistant.providers.ollama') },
])
const defaultUrl = computed(() => DEFAULT_URLS[form.provider] ?? DEFAULT_URLS.openai)
const modelError = computed(() =>
  touched.value && form.enabled && !form.model.trim() ? t('assistant.modelRequired') : '',
)

function buildPatch(): AssistantConfigPatch {
  const patch: AssistantConfigPatch = {
    enabled: form.enabled,
    provider: form.provider,
    base_url: form.baseUrl.trim(),
    model: form.model.trim(),
    temperature: form.temperature ?? 0.2,
    max_steps: form.maxSteps ?? 6,
    timeout_secs: form.timeoutSecs ?? 60,
  }
  // 留空 = 不改动已保存的密钥
  if (form.apiKey.trim()) patch.api_key = form.apiKey.trim()
  return patch
}

async function persist(patch: AssistantConfigPatch) {
  saving.value = true
  try {
    const res = await updateAssistantConfig(patch)
    form.apiKey = ''
    toast.success(t('assistant.saved'))
    emit('saved', res.data)
    return true
  } catch (err) {
    toast.error(err, t('common.failed'))
    return false
  } finally {
    saving.value = false
  }
}

async function save() {
  touched.value = true
  if (!props.canEdit || saving.value || modelError.value) return
  await persist(buildPatch())
}

async function clearKey() {
  await persist({ api_key: '' })
}

/** 先保存再测试：测试使用的是服务端已保存的配置 */
async function test() {
  if (testing.value) return
  touched.value = true
  if (modelError.value) return
  if (!(await persist(buildPatch()))) return
  testing.value = true
  try {
    const res = await testAssistant()
    const { models, model_found } = res.data
    if (model_found === false)
      toast.warning(t('assistant.testModelMissing', { model: form.model.trim() }))
    else toast.success(t('assistant.testOk', { n: models.length }))
  } catch (err) {
    toast.error(err, t('common.failed'))
  } finally {
    testing.value = false
  }
}
</script>

<style scoped>
.section-title {
  margin: 0 0 var(--fp-space-1);
  font-size: var(--fp-font-md, 15px);
}
.muted {
  color: var(--fp-text-muted);
  font-size: 13px;
  margin: 0 0 var(--fp-space-3);
}
.form {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
  margin-top: var(--fp-space-3);
}
.grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--fp-space-3);
}
.grid.three {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}
@media (max-width: 768px) {
  .grid,
  .grid.three {
    grid-template-columns: 1fr;
  }
}
.clear-key {
  align-self: flex-start;
}
.actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--fp-space-2);
  margin-top: var(--fp-space-4);
}
</style>
