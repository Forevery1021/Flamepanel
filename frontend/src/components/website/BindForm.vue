<template>
  <div class="modal-form">
    <FpSelect
      v-model="siteType"
      :label="t('website.siteType')"
      :options="siteTypeOptions"
      option-label="label"
      option-value="value"
    />
    <FpSelect
      v-if="siteType === 'runtime_php'"
      v-model="runtimeId"
      :label="t('website.bindRuntime')"
      :options="phpRuntimeOptions"
      option-label="label"
      option-value="value"
      show-clear
    />
    <FpSelect
      v-if="siteType === 'reverse_proxy'"
      v-model="projectId"
      :label="t('website.bindProject')"
      :options="projectOptions"
      option-label="label"
      option-value="value"
      show-clear
    />
    <FpSelect
      v-model="rewritePreset"
      :label="t('website.rewritePreset')"
      :options="rewritePresetOptions"
      option-label="label"
      option-value="value"
      show-clear
    />
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { SiteType } from '@/api/websites'
import FpSelect from '@/components/ui/FpSelect.vue'

/**
 * 站点绑定表单（Doc/23 §3.3 T2-4；M-B 主线二 Stage A-6 · Doc/20 §5.5）。
 *
 * 站点类型决定显示哪个绑定目标下拉（PHP 运行时 / 项目，两者互斥）；伪静态模板为
 * 可选追加项（选中后绑定成功即套用）。
 *
 * **两个绑定目标按 FpSelect 的 model 签名声明**（`string | number | string[] |
 * undefined`——「清空」经 FpSelect 上报为 `undefined`）：归一为后端协议所需的
 * `number | null` 由 `useWebsiteBinding.bind` 完成，展示层不必再关心 null/undefined
 * 差异（原实现用一对可空 computed 在视图里桥接，拆分后该转换下沉到数据层）。
 */
const siteType = defineModel<SiteType>({ required: true })
// 显式声明默认值（undefined = 未选择）：既消除 vue/require-default-prop 告警，
// 也把「清空」的语义标注在声明处——FpSelect 清空时上报 undefined 即回到此初值。
const runtimeId = defineModel<string | number | string[] | undefined>('runtimeId', {
  default: undefined,
})
const projectId = defineModel<string | number | string[] | undefined>('projectId', {
  default: undefined,
})
const rewritePreset = defineModel<string>('rewritePreset', { default: '' })

withDefaults(
  defineProps<{
    siteTypeOptions?: { label: string; value: SiteType }[]
    phpRuntimeOptions?: { label: string; value: number }[]
    projectOptions?: { label: string; value: number }[]
    rewritePresetOptions?: { label: string; value: string }[]
  }>(),
  {
    siteTypeOptions: () => [],
    phpRuntimeOptions: () => [],
    projectOptions: () => [],
    rewritePresetOptions: () => [],
  },
)

const { t } = useI18n()
</script>
