<template>
  <TerminalView :key="nodeId" :node-id="nodeId" :node-name="nodeName" />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import TerminalView from './TerminalView.vue'

/**
 * 节点远程交互式终端（Agent Edge，`/nodes/:id/terminal`）。
 *
 * 只做路由 → props 的适配：`:key` 绑定节点 ID，使在不同节点终端间跳转时整棵
 * TerminalView 重建（WS 端点随节点变化，`useWebSocket` 的 URL 在 setup 时确定）。
 */
const route = useRoute()
const nodeId = computed(() => Number(route.params.id))
const nodeName = computed(() => {
  const q = route.query.name
  return typeof q === 'string' && q ? q : `#${nodeId.value}`
})
</script>
