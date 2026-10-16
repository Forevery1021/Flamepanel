<template>
  <FpDrawer v-model="visible" :header="t('project.logsTitle', { name: project?.name ?? '' })">
    <div class="logs-body">
      <FpStatePanel
        :loading="loading"
        :error="error"
        :empty="lines.length === 0"
        retryable
        @retry="emit('refresh')"
      >
        <div class="logs-body__actions">
          <FpButton
            variant="ghost"
            size="small"
            icon="oi oi-refresh"
            :loading="fetching"
            @click="emit('refresh')"
          >
            {{ t('common.retry') }}
          </FpButton>
        </div>
        <pre class="logs-pre">{{ text }}</pre>
      </FpStatePanel>
    </div>
  </FpDrawer>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { ProjectDto } from '@/api/generated'
import FpDrawer from '@/components/ui/FpDrawer.vue'
import FpStatePanel from '@/components/ui/FpStatePanel.vue'
import FpButton from '@/components/ui/FpButton.vue'

/**
 * 项目日志抽屉（Doc/23 §3.3 T2-4 视图拆分第六站）。
 *
 * 三态交给 `FpStatePanel`（加载 / 错误可重试 / 空），正文是一段等宽 `<pre>`——
 * 日志不做行内高亮与虚拟滚动：后端固定 tail 200 行，量级不需要，而 `pre` 的
 * `white-space: pre-wrap` 保住了长行（栈追踪、JSON）的可读性且不撑破抽屉。
 *
 * 「重试」按钮放在正文上方而非三态面板的重试槽里：日志已有内容时的重拉
 * （面板在成功态不渲染 retry 槽）也要有入口——运维刷新日志是高频动作。
 *
 * 目标项目走 props（`project` 只用于标题），日志正文与三态由父级从数据层透传，
 * 本组件不持有任何请求状态。
 */
withDefaults(
  defineProps<{
    /** 当前查看的项目（仅用于标题；空值给空串，标题模板的 `{name}` 不会渲染成 undefined） */
    project?: ProjectDto | null
    /** 日志纯文本（行拼接在数据层完成） */
    text?: string
    /** 行数（空态判定：0 行即空） */
    lines?: unknown[]
    loading?: boolean
    /** 任意一次拉取中（重拉按钮的 loading；与 `loading` 分开——已有日志时重拉不闪骨架） */
    fetching?: boolean
    error?: string | null
  }>(),
  { project: null, text: '', lines: () => [], loading: false, fetching: false, error: null },
)

const visible = defineModel<boolean>({ required: true })

const emit = defineEmits<{ refresh: [] }>()

const { t } = useI18n()
</script>

<style scoped>
.logs-body {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-2);
  min-height: 200px;
}
.logs-body__actions {
  display: flex;
  justify-content: flex-end;
}
.logs-pre {
  margin: 0;
  padding: var(--fp-space-3);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-sm);
  font-family: var(--fp-font-mono);
  font-size: 12px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 60vh;
  overflow: auto;
}
</style>
