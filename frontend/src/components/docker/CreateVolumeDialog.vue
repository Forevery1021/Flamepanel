<template>
  <FpModal v-model="visible" :header="t('docker.createVolume')" size="md">
    <form class="modal-form" @submit.prevent="submit">
      <FpInput v-model="name" :label="t('docker.name')" :error="error" />
      <FpSelect
        v-model="driver"
        :label="t('docker.driver')"
        :options="driverOptions"
        option-label="label"
        option-value="value"
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
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { driverOptions as buildDriverOptions, VOLUME_DRIVERS } from '@/utils/dockerFormat'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'

/**
 * 创建卷弹窗（Doc/23 §3.3 T2-4 视图拆分第三站）。
 *
 * 与创建网络同形但少一个子网字段；驱动白名单是 `local` / `nfs`（卷支持的后端
 * 类型与网络不同，共用一份候选会立刻出错——故两处各取自己的常量）。
 */
withDefaults(
  defineProps<{
    submitting?: boolean
  }>(),
  { submitting: false },
)

const emit = defineEmits<{ submit: [payload: { name: string; driver: string }] }>()

const { t } = useI18n()

const visible = ref(false)
const name = ref('')
const driver = ref<string>('local')
const error = ref('')

const driverOptions = computed(() => buildDriverOptions(VOLUME_DRIVERS))

/**
 * 打开弹窗。**刻意不重置字段**——原实现在「提交成功」后才复位表单，
 * 取消后重开仍保留上次输入（本次拆分保持该语义不变，改动见 PR「后续建议」）。
 */
function open() {
  visible.value = true
}

function close() {
  visible.value = false
}

/**
 * 提交成功后的出口：先复位再关闭（对应原实现的「成功才清空表单」）。
 * 取消走 `close()` 不复位——两条路径分开，是为了不把「提交成功」的语义
 * 混进「关闭弹窗」这件事里。
 */
function closeAndReset() {
  reset()
  visible.value = false
}

function reset() {
  name.value = ''
  driver.value = 'local'
  error.value = ''
}

function submit() {
  if (!name.value.trim()) {
    error.value = t('common.required')
    return
  }
  emit('submit', { name: name.value, driver: driver.value })
}

defineExpose({ open, close, closeAndReset })
</script>
