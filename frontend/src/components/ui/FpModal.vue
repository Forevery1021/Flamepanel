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
    :class="`fp-modal--${size}`"
    :breakpoints="{ '768px': '92vw' }"
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
    /** 宽度档位：sm 420 / md 520 / lg 640 / xl 800 / full 96vw；仍可用 style 覆盖 */
    size?: 'sm' | 'md' | 'lg' | 'xl' | 'full'
  }>(),
  { header: '', closable: true, dismissableMask: true, size: 'md' },
)

const visible = defineModel<boolean>({ required: true })
</script>

<style>
/* Dialog 经 Teleport 挂到 body，scoped 选择器打不到根节点，故此处用全局样式（前缀 fp-modal 隔离） */
.p-dialog.fp-modal {
  border-radius: var(--fp-radius-lg);
  overflow: hidden;
  max-width: 96vw;
}
.p-dialog.fp-modal--sm {
  width: 420px;
}
.p-dialog.fp-modal--md {
  width: 520px;
}
.p-dialog.fp-modal--lg {
  width: 640px;
}
.p-dialog.fp-modal--xl {
  width: 800px;
}
.p-dialog.fp-modal--full {
  width: 96vw;
}
.fp-modal .fp-modal__body {
  padding: var(--fp-space-2) 0;
}
/* 手机：弹窗贴底成全宽 sheet，内容区可滚动，底部按钮始终可见（W4-a 响应式） */
@media (max-width: 640px) {
  .p-dialog.fp-modal {
    width: 100vw !important;
    max-width: 100vw;
    max-height: 92dvh;
    margin: auto 0 0;
    border-bottom-left-radius: 0;
    border-bottom-right-radius: 0;
  }
  .fp-modal .fp-modal__footer {
    flex-wrap: wrap;
  }
}
.fp-modal .fp-modal__footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--fp-space-2);
  padding-top: var(--fp-space-2);
}
</style>
