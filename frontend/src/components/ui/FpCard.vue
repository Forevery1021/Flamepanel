<template>
  <div class="fp-card">
    <Card class="fp-card__inner" :pt="{ body: { class: 'fp-card__body' } }">
      <template #title>
        <div class="fp-card-title">
          <span class="fp-card-title__bar" aria-hidden="true" />
          <span class="fp-card-title__text">{{ title }}</span>
          <span v-if="$slots.extra" class="fp-card-title__extra">
            <slot name="extra" />
          </span>
        </div>
      </template>
      <!--
        默认插槽内容必须落到 Card 的命名插槽 `content`。
        底层 `openvue/card` 的模板只渲染 `content` 命名插槽进 `p-card-content`，
        没有默认插槽出口——`<slot />` 会落进 Card 的默认插槽，**无处渲染**，
        内容被静默丢弃且不报错（T1-1 发现的真实缺陷，曾致 RuntimeView 卡片主体
        整体不可见）。此处显式转发到 `content`，为调用方保留默认插槽写法。
      -->
      <template #content>
        <slot />
      </template>
      <template v-if="$slots.footer" #footer>
        <slot name="footer" />
      </template>
    </Card>
  </div>
</template>

<script setup lang="ts">
import Card from 'openvue/card'

defineProps<{ title?: string }>()
</script>

<style scoped>
.fp-card__inner {
  height: 100%;
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-surface);
  backdrop-filter: var(--fp-surface-filter);
  -webkit-backdrop-filter: var(--fp-surface-filter);
  border-color: var(--fp-border);
}
.fp-card__body {
  padding: var(--fp-space-4);
}
.fp-card-title {
  display: flex;
  align-items: center;
  gap: var(--fp-space-2);
}
.fp-card-title__bar {
  width: 4px;
  height: 16px;
  border-radius: 2px;
  background: var(--fp-brand);
  flex-shrink: 0;
}
.fp-card-title__text {
  font-size: 14.5px;
  font-weight: 600;
  color: var(--fp-text-primary);
}
.fp-card-title__extra {
  margin-left: auto;
}
</style>
