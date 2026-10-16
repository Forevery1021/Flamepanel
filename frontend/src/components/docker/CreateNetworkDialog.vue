<template>
  <FpModal v-model="visible" :header="t('docker.createNetwork')" size="md">
    <form class="modal-form" @submit.prevent="submit">
      <FpInput v-model="name" :label="t('docker.name')" :error="error" />
      <FpSelect
        v-model="driver"
        :label="t('docker.driver')"
        :options="driverOptions"
        option-label="label"
        option-value="value"
      />
      <FpInput v-model="subnet" :label="t('docker.subnet')" placeholder="172.28.0.0/16" />
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
import { driverOptions as buildDriverOptions, NETWORK_DRIVERS } from '@/utils/dockerFormat'
import FpModal from '@/components/ui/FpModal.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'

/**
 * 创建网络弹窗（Doc/23 §3.3 T2-4 视图拆分第三站）。
 *
 * 三字段：名称（必填）、驱动（白名单下拉，默认 bridge）、子网（可选，占位示例
 * 给的是常见的 172.28.0.0/16）。子网留空时**不上报空串**——`createNetwork` 的
 * 第三参为可选，空串会被后端当成非法 CIDR，故提交时归一为 `undefined`。
 */
withDefaults(
  defineProps<{
    submitting?: boolean
  }>(),
  { submitting: false },
)

const emit = defineEmits<{ submit: [payload: { name: string; driver: string; subnet: string }] }>()

const { t } = useI18n()

const visible = ref(false)
const name = ref('')
const driver = ref<string>('bridge')
const subnet = ref('')
const error = ref('')

const driverOptions = computed(() => buildDriverOptions(NETWORK_DRIVERS))

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
  driver.value = 'bridge'
  subnet.value = ''
  error.value = ''
}

function submit() {
  if (!name.value.trim()) {
    error.value = t('common.required')
    return
  }
  emit('submit', { name: name.value, driver: driver.value, subnet: subnet.value })
}

defineExpose({ open, close, closeAndReset })
</script>
