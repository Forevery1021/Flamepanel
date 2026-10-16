<template>
  <Checkbox
    v-bind="$attrs"
    :model-value="model"
    :binary="binary"
    @update:model-value="emitModel"
    @change="onChange"
  />
</template>

<script setup lang="ts">
import Checkbox from 'openvue/checkbox'

withDefaults(
  defineProps<{
    binary?: boolean
  }>(),
  { binary: true },
)

const model = defineModel<boolean>({ default: false })
const emit = defineEmits<{ change: [value: boolean] }>()

function emitModel(v: boolean | undefined) {
  model.value = v ?? false
}

/**
 * C-3：底层 `change` 事件的载荷是 **DOM Event**（不是新值），`Boolean(event)` 恒为 `true`
 * ——取消勾选的信号会丢失（`MemoList` / Dashboard 待办的「取消完成」因此失效）。
 * 这里以事件源的 `checked` 为准：浏览器（与测试框架）在派发 change 之前已先行切换该状态，
 * 读到的即是本次交互后的新值；`change` 与 `update:modelValue` 两条通道由此同源。
 * 非 Event 载荷（底层实现变化时的兜底）保留旧回退，避免静默抛错。
 */
function onChange(v: unknown) {
  const source = v instanceof Event ? (v.target as HTMLInputElement | null) : null
  emit('change', source ? source.checked : Boolean(v))
}
</script>
