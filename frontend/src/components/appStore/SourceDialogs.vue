<template>
  <!-- 添加远程源：名称 / 类型 / 地址 / 验签公钥 -->
  <FpModal v-model="visible" :header="t('appStore.addSource')" size="md">
    <div class="modal-form">
      <FpInput v-model="form.name" :label="t('appStore.colSourceName')" />
      <FpSelect
        v-model="form.source_type"
        :label="t('appStore.colSourceType')"
        :options="sourceTypeOptions"
        option-label="label"
        option-value="value"
      />
      <FpInput
        v-model="form.url"
        :label="t('appStore.colSourceUrl')"
        :placeholder="urlPlaceholder"
      />
      <FpInput
        v-model="form.public_key"
        :label="t('appStore.sourcePublicKey')"
        :placeholder="t('appStore.sourcePublicKeyPlaceholder')"
      />
    </div>
    <template #footer>
      <FpButton variant="ghost" @click="visible = false">{{ t('common.cancel') }}</FpButton>
      <FpButton variant="primary" :loading="creating" @click="submitCreate">
        {{ t('appStore.addAndSync') }}
      </FpButton>
    </template>
  </FpModal>

  <!-- 公钥编辑：清空即卸载信任锚（确认分支在 useAppStoreSources） -->
  <FpModal
    v-model="keyVisible"
    :header="t('appStore.editPublicKey') + ' - ' + (source?.name ?? '')"
    size="md"
  >
    <div class="modal-form">
      <p class="security-hint">{{ t('appStore.editPublicKeyHint') }}</p>
      <FpInput
        v-model="keyForm.public_key"
        :label="t('appStore.sourcePublicKey')"
        :placeholder="t('appStore.sourcePublicKeyPlaceholder')"
      />
    </div>
    <template #footer>
      <FpButton variant="ghost" @click="keyVisible = false">{{ t('common.cancel') }}</FpButton>
      <FpButton variant="primary" :loading="savingKey" @click="submitKey">
        {{ t('common.save') }}
      </FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { computed, reactive, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { AppSource } from '@/api/appStore'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'

/**
 * 远程源的两个弹窗（Doc/23 §3.3 T2-4 视图拆分）。
 *
 * **编辑态归本组件**（本地 `reactive`），提交时把载荷上抛给
 * `useAppStoreSources.create/savePublicKey`——请求、错误文案与二次确认在那边，
 * 组件只负责「什么时候发、发什么」，避免跨组件 mutate 传入对象。
 */
const visible = defineModel<boolean>({ default: false })
const keyVisible = defineModel<boolean>('keyVisible', { default: false })

const props = withDefaults(
  defineProps<{
    creating?: boolean
    savingKey?: boolean
    source?: AppSource | null
  }>(),
  { creating: false, savingKey: false, source: null },
)

const emit = defineEmits<{
  create: [payload: { name: string; source_type: string; url: string; public_key: string }]
  saveKey: [value: string]
}>()

const { t } = useI18n()

const sourceTypeOptions = [
  { label: 'Git', value: 'git' },
  { label: 'HTTP', value: 'http' },
]

const form = reactive({
  name: '',
  source_type: 'git',
  url: '',
  public_key: '',
})

const keyForm = reactive({ public_key: '' })

const urlPlaceholder = computed(() =>
  form.source_type === 'git'
    ? 'https://github.com/1Panel-dev/appstore'
    : 'https://apps.example.com/manifest.json',
)

// 打开「添加」弹窗即重置表单（与拆分前 openDialog 语义一致：每次都是新建会话）
watch(visible, (open) => {
  if (!open) return
  form.name = ''
  form.source_type = 'git'
  form.url = ''
  form.public_key = ''
})

// 打开公钥弹窗时以当前行的公钥为初值。
// `immediate: true` 不可省：弹窗可能**带着可见态一起挂载**（父级先设 keyDialogVisible），
// 此时 watch 若不立即求值，初值永远是空串——「编辑不回显」正是这种隐性缺陷。
watch(
  () => [keyVisible.value, props.source] as const,
  ([open, source]) => {
    if (!open) return
    keyForm.public_key = source?.public_key ?? ''
  },
  { immediate: true },
)

function submitCreate() {
  emit('create', { ...form })
}

function submitKey() {
  emit('saveKey', keyForm.public_key)
}
</script>

<style scoped>
.security-hint {
  margin: 0;
  font-size: 12px;
  color: var(--fp-text-secondary);
  line-height: 1.6;
}
</style>
