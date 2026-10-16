<template>
  <FpModal v-model="visible" :header="`${t(headerKey)} — ${container}`" :style="{ width }">
    <div v-if="!json" class="text-muted">{{ emptyText }}</div>
    <pre v-else-if="variant === 'terminal'" class="json-viewer">{{ json }}</pre>
    <div v-else class="mono text-xs pre-wrap">{{ json }}</div>
  </FpModal>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import FpModal from '@/components/ui/FpModal.vue'

/**
 * 容器 JSON 预览弹窗（统计 / 详情共用一个组件，Doc/23 §3.3 T2-4 视图拆分第三站）。
 *
 * 统计与详情两个弹窗在结构上完全一致（标题 + 一段 JSON），差异只有三点：标题
 * 文案、宽度、空态文案。拆成两个组件会得到两份逐行相同的模板；合并为一个
 * 参数化组件后，**差异都成为显式 prop**，将来任一处改样式不再需要「记得改另一处」。
 *
 * 刻意不做 JSON 语法高亮：两个弹窗的用途是「把后端返回原样给运维看」，
 * 高亮需要引入额外依赖或自研解析，收益不成比例（原实现本就是纯文本）。
 */
withDefaults(
  defineProps<{
    headerKey: string
    container?: string
    json?: string
    emptyText?: string
    width?: string
    /**
     * 呈现变体：`plain` 为普通等宽文本（统计弹窗），`terminal` 为暗色终端底
     * （详情弹窗）。两者在原实现里就是两套样式，合并组件时刻意保留——
     * 「详情」给的是要与官方文档对照的完整 JSON，暗底更容易看出层级。
     */
    variant?: 'plain' | 'terminal'
  }>(),
  { container: '', json: '', emptyText: '', width: '700px', variant: 'plain' },
)

const visible = defineModel<boolean>({ default: false })

const { t } = useI18n()
</script>

<style scoped>
.json-viewer {
  background: var(--fp-bg-terminal);
  color: var(--fp-text-code);
  padding: 16px;
  border-radius: 6px;
  max-height: 500px;
  overflow: auto;
  font-size: 13px;
  line-height: 1.5;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
