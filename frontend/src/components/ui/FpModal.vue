<template>
  <Dialog
    v-model:visible="visible"
    v-bind="$attrs"
    :header="header"
    :modal="true"
    :closable="closable"
    :dismissable-mask="dismissableMask"
    :draggable="false"
    class="fp-modal"
  >
    <div class="fp-modal__body">
      <slot />
    </div>
    <template v-if="$slots.footer" #footer>
      <div class="fp-modal__footer">
        <slot name="footer" />
      </div>
    </template>
  </Dialog>
</template>

<script setup lang="ts">
import Dialog from 'openvue/dialog'

withDefaults(
  defineProps<{
    header?: string
    closable?: boolean
    dismissableMask?: boolean
  }>(),
  { header: '', closable: true, dismissableMask: true },
)

const visible = defineModel<boolean>({ required: true })
</script>

<style scoped>
.fp-modal {
  border-radius: var(--fp-radius-lg);
  overflow: hidden;
}
.fp-modal__body {
  padding: var(--fp-space-2) 0;
}
.fp-modal__footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--fp-space-2);
  padding-top: var(--fp-space-2);
}
</style>
