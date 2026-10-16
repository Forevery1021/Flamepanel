<template>
  <FpModal v-model="visible" :header="t('docker.rename')" size="sm">
    <form class="modal-form" @submit.prevent="submit">
      <FpInput
        v-model="target"
        :label="t('docker.name')"
        :error="error"
        :placeholder="t('docker.renamePlaceholder')"
      />
    </form>
    <template #footer>
      <FpButton variant="ghost" @click="close">{{ t('common.cancel') }}</FpButton>
      <FpButton variant="primary" :loading="submitting" @click="submit">{{
        t('common.confirm')
      }}</FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { DockerContainer } from '@/types'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpButton from '@/components/ui/FpButton.vue'

/**
 * 容器重命名弹窗（Doc/23 §3.3 T2-4 视图拆分第三站）。
 *
 * 编辑态（当前输入、校验错误）归本组件，提交时上抛 `(id, name)`；
 * 请求与「成功才关」由父级编排（失败保持打开让用户改输入——原实现如此）。
 *
 * 表单包 `@submit.prevent`：回车即提交（原实现已有），键盘用户不必去够按钮。
 */
withDefaults(
  defineProps<{
    submitting?: boolean
  }>(),
  { submitting: false },
)

const emit = defineEmits<{ submit: [id: string, name: string] }>()

const { t } = useI18n()

const visible = ref(false)
const target = ref('')
const error = ref('')
/** 目标容器 ID 是**会话级**数据：打开时快照，用户改输入不影响它 */
let targetId = ''

/** 打开弹窗：以当前行为初值（名称可能为空——后端允许无名容器，此时从空串开始） */
function open(row: DockerContainer) {
  targetId = row.id
  target.value = row.name
  error.value = ''
  visible.value = true
}

function close() {
  visible.value = false
}

function submit() {
  if (!target.value.trim()) {
    error.value = t('common.required')
    return
  }
  emit('submit', targetId, target.value.trim())
}

defineExpose({ open, close })
</script>
