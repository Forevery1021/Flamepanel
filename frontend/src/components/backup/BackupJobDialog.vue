<template>
  <FpModal v-model="visible" :header="header" size="md">
    <div class="modal-form">
      <FpInput v-model="form.name" :label="t('backup.jobName')" :error="errors.name" />
      <FpSelect
        v-model="form.scope"
        :label="t('backup.jobScope')"
        :options="scopeOptions"
        option-label="label"
        option-value="value"
      />
      <FpInput
        v-if="form.scope === 'app'"
        v-model="form.scope_key"
        :label="t('backup.jobScopeKey')"
        :error="errors.scope_key"
        placeholder="my-app"
      />
      <FpInput
        v-model="form.schedule"
        :label="t('backup.jobSchedule')"
        :error="errors.schedule"
        placeholder="0 3 * * *"
      />
      <FpSelect
        v-model="form.target_account_id"
        :label="t('backup.jobTarget')"
        :options="accountOptions"
        option-label="label"
        option-value="value"
        :error="errors.target_account_id"
      />
      <FpNumber v-model="form.retention" :label="t('backup.jobRetention')" :min="1" :max="999" />
      <FpSwitch v-model="form.enabled" :label="t('backup.jobEnabled')" />
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
import type { BackupJobDto } from '@/api/generated'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpNumber from '@/components/ui/FpNumber.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import FpButton from '@/components/ui/FpButton.vue'

/**
 * 计划任务创建 / 编辑弹窗（Doc/23 §3.3 T2-4 视图拆分第四站）。
 *
 * 三条本地契约：
 *
 * 1. **`scope_key` 只在 app 范围渲染**（不是渲染了禁用）——面板库备份没有
 *    「应用 key」这个概念，把它摆在那里会让人以为要填；校验也随范围分流。
 * 2. **新建的任务默认挂在第一个启用账号上**（`accountOptions[0]`）：绝大多数
 *    场景下用户只有一个账号，省一步选择；账号列表为空时保持 0，由校验拦下
 *    并给红字（而不是让提交打到后端换一个 400）。
 * 3. **本地校验四项齐发**——不「发现一个错就返回」，一次性把问题都标出来。
 *
 * 编辑态回填保留原实现语义：`retention` 走 `?? 7`（**只兜 null/undefined**，
 * 后端约定 `0 = 用默认 7` 的 0 原样保留、原样回传）。若在这里把 0 归一成 7，
 * 用户每次编辑都会「悄悄改变」一个他没动过的字段；该 0 值是否该在 UI 上做
 * 归一展示属产品决策（见 PR「后续建议」）。
 */
const props = withDefaults(
  defineProps<{
    submitting?: boolean
    /** 目标账号候选（只列启用账号，由数据层派生） */
    accountOptions?: { label: string; value: number }[]
  }>(),
  { submitting: false, accountOptions: () => [] },
)

const emit = defineEmits<{
  submit: [
    payload: {
      name: string
      scope: string
      scope_key: string
      schedule: string
      target_account_id: number
      retention: number
      enabled: boolean
    },
  ]
}>()

const { t } = useI18n()

const visible = ref(false)
const editing = ref(false)

const form = reactive({
  name: '',
  scope: 'panel',
  scope_key: '',
  schedule: '0 3 * * *',
  target_account_id: 0,
  retention: 7,
  enabled: true,
})
const errors = reactive({ name: '', scope_key: '', schedule: '', target_account_id: '' })

const header = computed(() => (editing.value ? t('backup.jobEdit') : t('backup.jobCreate')))
const scopeOptions = computed(() => [
  { label: t('backup.scopePanel'), value: 'panel' },
  { label: t('backup.scopeApp'), value: 'app' },
])

function open(row?: BackupJobDto) {
  editing.value = !!row
  form.name = row?.name ?? ''
  form.scope = row?.scope ?? 'panel'
  form.scope_key = row?.scope_key ?? ''
  form.schedule = row?.schedule ?? '0 3 * * *'
  // 新建默认挂第一个启用账号；编辑沿用行值（账号可能已停用，但保留原配置）
  form.target_account_id = row?.target_account_id ?? props.accountOptions[0]?.value ?? 0
  form.retention = row?.retention ?? 7
  form.enabled = row?.enabled ?? true
  errors.name = ''
  errors.scope_key = ''
  errors.schedule = ''
  errors.target_account_id = ''
  visible.value = true
}

function close() {
  visible.value = false
}

function submit() {
  errors.name = form.name ? '' : t('common.required')
  errors.schedule = form.schedule ? '' : t('common.required')
  errors.scope_key = form.scope === 'app' && !form.scope_key ? t('common.required') : ''
  errors.target_account_id = form.target_account_id > 0 ? '' : t('common.required')
  if (errors.name || errors.schedule || errors.scope_key || errors.target_account_id) return
  emit('submit', {
    name: form.name,
    scope: form.scope,
    scope_key: form.scope_key,
    schedule: form.schedule,
    target_account_id: form.target_account_id,
    retention: form.retention ?? 7,
    enabled: form.enabled,
  })
}

defineExpose({ open, close })
</script>
