<template>
  <Drawer
    v-model:visible="visible"
    v-bind="$attrs"
    :header="header"
    :position="position"
    :modal="true"
    class="fp-drawer"
  >
    <div class="fp-drawer__body">
      <slot />
    </div>
    <template v-if="$slots.footer" #footer>
      <div class="fp-drawer__footer">
        <slot name="footer" />
      </div>
    </template>
  </Drawer>
</template>

<script setup lang="ts">
import Drawer from 'openvue/drawer'

withDefaults(
  defineProps<{
    header?: string
    position?: 'left' | 'right' | 'top' | 'bottom'
  }>(),
  { header: '', position: 'right' },
)

const visible = defineModel<boolean>({ required: true })
</script>

<style scoped>
.fp-drawer {
  border-radius: var(--fp-radius-lg);
  overflow: hidden;
}
.fp-drawer__body {
  padding: var(--fp-space-2) 0;
}
.fp-drawer__footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--fp-space-2);
  padding-top: var(--fp-space-2);
}
</style>
