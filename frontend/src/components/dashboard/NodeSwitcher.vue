<template>
  <FpSelect
    v-if="visible"
    :model-value="modelValue ?? LOCAL"
    :options="options"
    option-label="label"
    option-value="value"
    class="node-switcher"
    :aria-label="t('dashboard.node.label')"
    @update:model-value="onChange"
  />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import FpSelect from '@/components/ui/FpSelect.vue'
import { useApiQuery } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import { listNodes } from '@/api/nodes'
import type { ServerNode } from '@/types'

/**
 * 仪表盘节点切换（Doc/30 M-P）：本机 + 已注册节点。远程节点的指标来自 Agent 心跳并入的
 * 每节点 MetricsHub（`/ws/metrics?node_id=`）。没有任何节点时整体隐藏——单机部署不多一个
 * 无意义的下拉框；无 `node:read` 权限时不发请求（`enabled`）。
 */
const props = defineProps<{
  /** 当前节点 ID；null 表示本机 */
  modelValue: number | null
  /** 是否允许读取节点列表（持有 node:read） */
  enabled: boolean
}>()
const emit = defineEmits<{ 'update:modelValue': [id: number | null] }>()

/** 本机在下拉框里的占位值（FpSelect 的值类型不含 null） */
const LOCAL = 0
/** 仪表盘切换器只列前 100 个节点；更多节点从「节点管理」的「监控」入口进入 */
const PAGE_SIZE = 100

const { t } = useI18n()

const query = useApiQuery<ServerNode[]>(
  () => [...queryKeys.nodes.list(1, PAGE_SIZE), 'switcher'],
  async () => {
    const res = await listNodes(1, PAGE_SIZE)
    return { data: res.data.data }
  },
  { enabled: () => props.enabled },
)

const nodes = computed(() => query.data.value ?? [])
// 直接经 URL 进入某节点（如节点列表「监控」）时，即便列表尚未返回也保持可切回本机
const visible = computed(() => nodes.value.length > 0 || props.modelValue !== null)

const options = computed(() => [
  { label: t('dashboard.node.local'), value: LOCAL },
  ...nodes.value.map((n) => ({ label: `${n.name} (${n.ip_address})`, value: n.id })),
])

function onChange(value: string | number | string[] | undefined) {
  const id = typeof value === 'number' && value !== LOCAL ? value : null
  emit('update:modelValue', id)
}
</script>

<style scoped>
.node-switcher {
  min-width: 220px;
}
</style>
