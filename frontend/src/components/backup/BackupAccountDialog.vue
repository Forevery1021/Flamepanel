<template>
  <FpModal v-model="visible" :header="header" size="md">
    <div class="modal-form">
      <FpInput v-model="form.name" :label="t('backup.accountName')" :error="errors.name" />
      <FpSelect
        v-model="form.account_type"
        :label="t('backup.accountType')"
        :options="typeOptions"
        option-label="label"
        option-value="value"
      />
      <div class="field-col">
        <FpTextarea
          v-model="form.config_json"
          :label="t('backup.accountConfig')"
          :rows="5"
          :invalid="!!errors.config_json"
          :placeholder="placeholder"
        />
        <small v-if="errors.config_json" class="field-error">{{ errors.config_json }}</small>
        <small v-else-if="editing" class="field-hint">{{
          t('backup.accountConfigEditHint')
        }}</small>
      </div>
      <FpSwitch v-model="form.enabled" :label="t('backup.accountEnabled')" />
    </div>
    <template #footer>
      <FpButton variant="ghost" @click="visible = false">{{ t('common.cancel') }}</FpButton>
      <FpButton variant="primary" :loading="submitting" @click="submit">
        {{ t('common.save') }}
      </FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { BackupAccountDto } from '@/api/generated'
import { accountConfigPlaceholder, accountTypeOptions } from '@/utils/backupFormat'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpTextarea from '@/components/ui/FpTextarea.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import FpButton from '@/components/ui/FpButton.vue'

/**
 * 备份账号创建 / 编辑弹窗（Doc/23 §3.3 T2-4 视图拆分第四站）。
 *
 * 三条本地契约（原实现即有，拆分后逐条保留）：
 *
 * 1. **新建必填连接配置、编辑留空 = 不修改**——后端从不回显敏感配置，编辑时
 *    该字段永远是空的；若按「必填」处理，用户连改名都做不到。故校验随
 *    `editing` 分流，并在编辑态给出「留空表示不修改」的提示（红字与提示
 *    互斥显示，同一时刻只讲一件事）。
 * 2. **本地校验给字段旁红字，不静默返回**——数据层不做空值检查，静默返回会让
 *    用户以为「点了没反应」。
 * 3. **切换账号类型时不清空已填配置**——config_json 是手写 JSON，切错了再切回来
 *    不该丢内容；样例（placeholder）随类型变，已填内容保持。
 */
const props = withDefaults(
  defineProps<{
    /** 提交态（父级持有：请求在数据层，loading 回传） */
    submitting?: boolean
    /** 账号类型候选（默认取自纯函数层，测试可注入） */
    typeOptions?: { label: string; value: string }[]
  }>(),
  { submitting: false, typeOptions: undefined },
)

const emit = defineEmits<{
  submit: [payload: { name: string; account_type: string; config_json: string; enabled: boolean }]
}>()

const { t } = useI18n()

const visible = ref(false)
const editing = ref(false)

const form = reactive({
  name: '',
  account_type: 'local',
  config_json: '',
  enabled: true,
})
const errors = reactive({ name: '', config_json: '' })

const typeOptions = computed(() => props.typeOptions ?? accountTypeOptions(t))
const placeholder = computed(() => accountConfigPlaceholder(form.account_type))
const header = computed(() => (editing.value ? t('backup.accountEdit') : t('backup.accountCreate')))

/** 打开弹窗：新建给默认值，编辑以行数据回填（`config_json` 刻意留空） */
function open(row?: BackupAccountDto) {
  editing.value = !!row
  form.name = row?.name ?? ''
  form.account_type = row?.account_type ?? 'local'
  // 编辑时留空 = 不修改配置（后端不回显敏感 config_json）
  form.config_json = ''
  form.enabled = row?.enabled ?? true
  errors.name = ''
  errors.config_json = ''
  visible.value = true
}

function close() {
  visible.value = false
}

function submit() {
  errors.name = form.name ? '' : t('common.required')
  // 新建时必须提供连接配置；编辑时留空表示不修改
  errors.config_json = !editing.value && !form.config_json.trim() ? t('common.required') : ''
  if (errors.name || errors.config_json) return
  emit('submit', {
    name: form.name,
    account_type: form.account_type,
    config_json: form.config_json,
    enabled: form.enabled,
  })
}

defineExpose({ open, close })
</script>
