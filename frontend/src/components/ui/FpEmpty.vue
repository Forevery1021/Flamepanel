<template>
  <div class="fp-empty">
    <div class="fp-empty__icon">
      <i :class="icon" />
    </div>
    <p v-if="title" class="fp-empty__title">{{ title }}</p>
    <p v-if="description" class="fp-empty__desc">{{ description }}</p>
    <div v-if="$slots.action" class="fp-empty__action">
      <slot name="action" />
    </div>
  </div>
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    icon?: string
    title?: string
    description?: string
  }>(),
  { icon: 'oi oi-inbox', title: '', description: '' },
)
</script>

<style scoped>
.fp-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--fp-space-2);
  padding: var(--fp-space-8) var(--fp-space-4);
  text-align: center;
}
.fp-empty__icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 56px;
  height: 56px;
  border-radius: 999px;
  background: var(--fp-bg-hover);
  color: var(--fp-text-muted);
  font-size: 24px;
  margin-bottom: var(--fp-space-2);
}
.fp-empty__title {
  font-size: 15px;
  font-weight: 600;
  color: var(--fp-text-primary);
}
.fp-empty__desc {
  font-size: 13px;
  color: var(--fp-text-secondary);
  max-width: 320px;
}
.fp-empty__action {
  margin-top: var(--fp-space-2);
}
</style>
