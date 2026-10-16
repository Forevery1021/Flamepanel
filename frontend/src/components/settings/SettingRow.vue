<template>
  <div class="setting-row" :class="{ 'setting-row--block': block }">
    <div class="setting-row__meta">
      <label v-if="label" :id="labelId" class="setting-row__label" :for="controlId">
        {{ label }}
        <span v-if="badge" class="setting-row__badge">{{ badge }}</span>
      </label>
      <p v-if="description || $slots.description" class="setting-row__desc">
        <slot name="description">{{ description }}</slot>
      </p>
    </div>
    <div class="setting-row__control">
      <slot :id="controlId" :label-id="labelId" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { useId } from 'vue'

/**
 * 设置行：左「标题 + 说明」、右「紧凑控件」。
 *
 * 参照 1Panel / macOS 系统设置的行式布局——比「表单字段堆叠」更易扫读，控件也不必撑满整行。
 * 容器宽度 < 560px（窄屏 / 分栏）时自动改为上下堆叠（容器查询，由 `.settings-panel` 声明容器）。
 *
 * 可访问性：默认插槽拿到 `id`（传给 Fp* 的 `input-id`）与 `labelId`，标题即控件的 `<label for>`。
 */
withDefaults(
  defineProps<{
    label?: string
    description?: string
    /** 小徽标，如「需重启」「只读」 */
    badge?: string
    /** 控件区占满整行宽（textarea 等），窄屏外也放到标题下方 */
    block?: boolean
  }>(),
  { label: '', description: '', badge: '', block: false },
)

const controlId = useId()
const labelId = `${controlId}-label`
</script>

<style scoped>
.setting-row {
  display: grid;
  grid-template-columns: minmax(200px, 5fr) minmax(0, 6fr);
  align-items: center;
  gap: var(--fp-space-2) var(--fp-space-6);
  padding: var(--fp-space-3) 0;
  border-top: 1px solid var(--fp-border-subtle, var(--fp-border));
}
.setting-row:first-child {
  border-top: 0;
}
.setting-row--block {
  grid-template-columns: minmax(0, 1fr);
  align-items: stretch;
}
.setting-row__meta {
  min-width: 0;
}
.setting-row__label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: var(--fp-font-sm);
  font-weight: 600;
  color: var(--fp-text-primary);
}
.setting-row__badge {
  padding: 0 6px;
  border-radius: 999px;
  font-size: 11px;
  line-height: 18px;
  font-weight: 500;
  color: var(--fp-text-secondary);
  background: var(--fp-bg-subtle);
  border: 1px solid var(--fp-border);
}
.setting-row__desc {
  margin: 2px 0 0;
  font-size: var(--fp-font-xs);
  line-height: 1.5;
  color: var(--fp-text-muted);
}
.setting-row__control {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  flex-wrap: wrap;
  gap: var(--fp-space-2);
  min-width: 0;
}
/* 控件取紧凑宽度：输入框不超过 360px，右对齐 */
.setting-row__control > :deep(.fp-field),
.setting-row__control > :deep(.fp-number) {
  width: min(100%, 360px);
}
.setting-row--block .setting-row__control {
  justify-content: stretch;
}
.setting-row--block .setting-row__control > :deep(.fp-field) {
  width: 100%;
}

@container settings (max-width: 560px) {
  .setting-row {
    grid-template-columns: minmax(0, 1fr);
    align-items: stretch;
  }
  .setting-row__control {
    justify-content: flex-start;
  }
  .setting-row__control > :deep(.fp-field),
  .setting-row__control > :deep(.fp-number) {
    width: 100%;
  }
}
</style>
