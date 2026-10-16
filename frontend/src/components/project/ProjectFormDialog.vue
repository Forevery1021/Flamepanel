<template>
  <FpModal v-model="visible" :header="editing ? t('project.editTitle') : t('project.createTitle')">
    <div class="modal-form">
      <FpInput
        v-model="form.name"
        :label="t('project.name')"
        :placeholder="t('project.namePlaceholder')"
        :disabled="editing"
      />
      <FpSelect
        v-model="form.kind"
        :label="t('project.kind')"
        :options="kindSelectOptions(t)"
        option-label="label"
        option-value="value"
        placeholder=""
      />
      <FpSelect
        v-model="form.source"
        :label="t('project.source')"
        :options="sourceSelectOptions(t)"
        option-label="label"
        option-value="value"
        placeholder=""
      />
      <FpInput
        v-if="form.source === 'git'"
        v-model="form.source_url"
        :label="t('project.sourceUrl')"
        :placeholder="t('project.sourceUrlPlaceholder')"
      />
      <FpSelect
        v-model="runtimeIdModel"
        :label="t('project.runtime')"
        :options="runtimeOptions"
        option-label="label"
        option-value="value"
        show-clear
        placeholder=""
      />
      <FpInput
        v-model="form.run_command"
        :label="t('project.runCommand')"
        :placeholder="t('project.runCommandPlaceholder')"
      />
      <FpNumber v-model="form.port" :label="t('project.port')" :min="1" :max="65535" />
      <FpInput v-model="form.domain" :label="t('project.domain')" />
      <FpSelect
        v-model="form.deploy_strategy"
        :label="t('project.deployStrategy')"
        :options="strategySelectOptions(t)"
        option-label="label"
        option-value="value"
        placeholder=""
      />
      <FpInput
        v-if="form.deploy_strategy === 'rolling'"
        v-model="form.health_check_path"
        :label="t('project.healthCheckPath')"
        :placeholder="t('project.healthCheckPathPlaceholder')"
      />
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
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ProjectDto } from '@/api/generated'
import {
  initProjectForm,
  kindSelectOptions,
  projectFormFrom,
  sourceSelectOptions,
  strategySelectOptions,
  type ProjectFormState,
} from '@/utils/projectFormat'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpNumber from '@/components/ui/FpNumber.vue'
import FpButton from '@/components/ui/FpButton.vue'

/**
 * 项目创建 / 编辑弹窗（Doc/23 §3.3 T2-4 视图拆分第六站）。
 *
 * 三条本地契约（原实现即有，拆分后逐条保留）：
 *
 * 1. **编辑态 `name` 置灰**（`:disabled="editing"`）——项目名即部署目录名，
 *    改名会让旧目录成为孤儿；后端更新接口也刻意不收 `name`（见 `buildProjectBody`）。
 *    只是置灰而非隐藏：用户需要看到自己在改哪个项目。
 * 2. **`source === 'git'` 才显示仓库地址**——本地目录项目没有 URL 可填。
 * 3. **`deploy_strategy === 'rolling'` 才显示健康检查路径**——非 rolling 策略下
 *    没有切换探测窗口，路径填了也不会被使用（显示出来只会让人以为配了就生效）。
 *
 * `runtimeIdModel`：FpSelect 的 model 是 `string | number | undefined`（清空上报
 * `undefined`），而协议里 `runtime_id` 是 `number | null`。这层适配关在弹窗内部——
 * 数据层拿到的一律是协议形状，展示层不必理解两个空值语义的差异（原实现用视图里的
 * 一对可空 computed 桥接，本次下沉到真正使用它的组件）。
 */
withDefaults(
  defineProps<{
    /** 运行时下拉候选（数据层按 kind 去重后传入） */
    runtimeOptions?: { label: string; value: number }[]
    /** 提交态（父级持有：请求在数据层，loading 回传） */
    submitting?: boolean
  }>(),
  { runtimeOptions: () => [], submitting: false },
)

const emit = defineEmits<{
  create: [form: ProjectFormState]
  update: [id: number, form: ProjectFormState]
}>()

const { t } = useI18n()

const visible = ref(false)
const editing = ref(false)
/** 编辑目标 id（0 = 新建；由父级在 submit 时读回，避免父级再存一份） */
const editingId = ref(0)
const form = ref<ProjectFormState>(initProjectForm())

const runtimeIdModel = computed<number | undefined>({
  get: () => form.value.runtime_id ?? undefined,
  set: (v) => {
    form.value.runtime_id = (v ?? null) as number | null
  },
})

/** 打开弹窗：新建给初值，编辑以行数据回填 */
function open(row?: ProjectDto) {
  editing.value = !!row
  editingId.value = row?.id ?? 0
  form.value = row ? projectFormFrom(row) : initProjectForm()
  visible.value = true
}

function close() {
  visible.value = false
}

function submit() {
  if (editingId.value) {
    emit('update', editingId.value, form.value)
  } else {
    emit('create', form.value)
  }
}

defineExpose({ open, close })
</script>
