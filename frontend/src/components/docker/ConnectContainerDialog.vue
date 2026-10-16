<template>
  <FpModal v-model="visible" :header="t('docker.connectContainer')" size="md">
    <form class="modal-form" @submit.prevent="submit">
      <FpSelect
        v-model="containerId"
        :label="t('docker.container')"
        :options="options"
        option-label="label"
        option-value="value"
        filter
        :error="error"
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
import type { DockerNetwork } from '@/types'
import FpModal from '@/components/ui/FpModal.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpButton from '@/components/ui/FpButton.vue'

/**
 * 连接容器到网络弹窗（Doc/23 §3.3 T2-4 视图拆分第三站）。
 *
 * 容器候选由父级传入（来自容器页签的列表缓存，`{ label: 名称, value: ID }`）；
 * `filter` 打开因为候选可能很长。必填校验在本地拦一道——`useDockerNetworks.connect`
 * 也会对空值直接返回 false，两层各自成立：前者给用户即时反馈，后者保证
 * 「谁调用都不会发出无效请求」。
 */
withDefaults(
  defineProps<{
    options: { label: string; value: string }[]
    submitting?: boolean
  }>(),
  { submitting: false },
)

const emit = defineEmits<{ submit: [networkId: string, containerId: string] }>()

const { t } = useI18n()

const visible = ref(false)
const containerId = ref<string | undefined>(undefined)
const error = ref('')
let networkId = ''

function open(row: DockerNetwork) {
  networkId = row.id
  containerId.value = undefined
  error.value = ''
  visible.value = true
}

function close() {
  visible.value = false
}

function submit() {
  if (!containerId.value) {
    error.value = t('common.required')
    return
  }
  emit('submit', networkId, containerId.value)
}

defineExpose({ open, close })
</script>
