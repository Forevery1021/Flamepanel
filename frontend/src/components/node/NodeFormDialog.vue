<template>
  <FpModal v-model="visible" :header="editing ? t('node.editNode') : t('node.register')" size="md">
    <div class="modal-form">
      <FpInput v-model="form.name" :label="t('node.name')" :error="errors.name" />
      <FpInput v-model="form.hostname" :label="t('node.hostname')" :error="errors.hostname" />
      <FpInput v-model="form.ip_address" :label="t('node.ip')" :error="errors.ip_address" />
      <FpSelect
        v-model="form.status"
        :label="t('node.status')"
        :options="statusOptions"
        option-label="label"
        option-value="value"
      />
    </div>
    <template #footer>
      <FpButton variant="ghost" @click="visible = false">{{ t('common.cancel') }}</FpButton>
      <FpButton variant="primary" :loading="submitting" @click="submit">
        {{ t('common.confirm') }}
      </FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ServerNode } from '@/types'
import {
  hasFieldErrors,
  initNodeForm,
  nodeFormFrom,
  nodeStatusOptions,
  validateNodeFields,
  type NodeFieldErrors,
  type NodeFormState,
} from '@/utils/nodeFormat'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'

/**
 * 节点注册 / 编辑弹窗（Doc/23 §3.3 T2-4 视图拆分第七站）。
 *
 * 原文件里这是**两个几乎逐行相同的弹窗**（同一组四个字段、两份相同的校验函数、
 * 两段相同的提交前处理），差别只有标题与提交目标。本次合并为一个组件 + 两种
 * 打开姿势（`open()` / `open(row)`），把「同一份表单写两遍」这个维护陷阱从结构上
 * 消除：以后加一个字段只需改一处，不会再出现「注册能填、编辑忘了」。
 *
 * 三条本地契约（原实现即有，拆分后逐条保留）：
 *
 * 1. **本地校验给字段旁的红字**（`validateNodeFields` 纯函数，与数据层共用同一
 *    口径）；数据层不做格式校验——`hostname` 允许内网短名、`ip_address` 允许
 *    主机名，格式收紧会挡住真实可用的内网地址（见 `utils/nodeFormat` 注释）；
 * 2. **新建给初值、编辑以行数据回填**，且回填只取四个可编辑字段
 *    （`nodeFormFrom`）——`metrics_json` / 心跳快照是 Agent 上报的只读事实；
 * 3. **成功由父级关窗**（`create` / `update` 返回 `true` 才关），失败留在弹窗里
 *    让用户改——只 toast 不关窗是原实现口径。
 */
withDefaults(
  defineProps<{
    /** 提交态（父级持有：请求在数据层，loading 回传） */
    submitting?: boolean
  }>(),
  { submitting: false },
)

const emit = defineEmits<{
  create: [form: NodeFormState]
  update: [id: number, form: NodeFormState]
}>()

const { t } = useI18n()

const visible = ref(false)
const editing = ref(false)
/** 编辑目标 id（0 = 注册；由本组件在 submit 时读回，父级不再存一份） */
const editingId = ref(0)
const form = reactive<NodeFormState>(initNodeForm())
const errors = reactive<NodeFieldErrors>({ name: '', hostname: '', ip_address: '' })

const statusOptions = computed(() => nodeStatusOptions(t))

function clearErrors() {
  Object.assign(errors, { name: '', hostname: '', ip_address: '' })
}

/** 打开：无参 = 注册（给初值）；带行 = 编辑（回填四个可编辑字段） */
function open(row?: ServerNode) {
  editing.value = !!row
  editingId.value = row?.id ?? 0
  Object.assign(form, row ? nodeFormFrom(row) : initNodeForm())
  clearErrors()
  visible.value = true
}

function close() {
  visible.value = false
}

function submit() {
  Object.assign(errors, validateNodeFields(form, t))
  if (hasFieldErrors(errors)) return
  if (editingId.value) {
    emit('update', editingId.value, { ...form })
  } else {
    emit('create', { ...form })
  }
}

defineExpose({ open, close })
</script>
