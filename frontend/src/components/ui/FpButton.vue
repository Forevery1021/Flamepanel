<template>
  <Button
    v-bind="btnAttrs"
    class="fp-btn"
    :class="{ 'fp-btn--has-content': hasContent }"
    :aria-label="hasContent ? undefined : ($attrs['aria-label'] as string | undefined)"
  >
    <template v-if="hasContent">
      <i
        v-if="icon && !loading"
        :class="[
          'p-button-icon',
          icon,
          iconPos === 'right' ? 'p-button-icon-right' : 'p-button-icon-left',
        ]"
        data-pc-section="icon"
      />
      <span class="p-button-label" data-pc-section="label"><slot /></span>
    </template>
    <slot v-else />
  </Button>
</template>

<script setup lang="ts">
import { computed, useSlots } from 'vue'
import Button from 'openvue/button'
import type { ButtonHTMLAttributes } from 'vue'

type FpSeverity = 'primary' | 'secondary' | 'danger' | 'success' | 'warning' | 'ghost' | 'link'

const props = withDefaults(
  defineProps<{
    /** 语义化变体：映射到 OpenVue severity */
    variant?: FpSeverity
    label?: string
    icon?: string
    iconPos?: 'left' | 'right'
    size?: 'small' | 'large'
    loading?: boolean
    disabled?: boolean
    type?: ButtonHTMLAttributes['type']
    plain?: boolean
    rounded?: boolean
    outlined?: boolean
    text?: boolean
    title?: string
  }>(),
  {
    label: '',
    icon: '',
    title: '',
    variant: 'primary',
    size: 'small',
    loading: false,
    disabled: false,
    iconPos: 'left',
    type: 'button',
    plain: false,
    rounded: false,
    outlined: false,
    text: false,
  },
)

const severityMap: Record<FpSeverity, string | undefined> = {
  primary: 'primary',
  secondary: 'secondary',
  danger: 'danger',
  success: 'success',
  warning: 'warn',
  ghost: 'secondary',
  link: undefined,
}

/**
 * D1 修复：OpenVue Button 仅凭 `icon && !label` 判定 icon-only（不感知默认插槽），
 * 导致「icon + 插槽文字」的按钮被压成 32px 方块。有插槽内容时自行渲染图标 + label，
 * 并向 Button 传 `label=" "` 让其退出 icon-only 分支（视觉由我们的插槽覆盖）。
 */
const slots = useSlots()
const hasContent = computed(() => !props.label && !!slots.default)

const btnAttrs = computed(() => ({
  severity: severityMap[props.variant],
  label: hasContent.value ? ' ' : props.label,
  icon: props.icon,
  iconPos: props.iconPos,
  size: props.size === 'small' ? 'small' : undefined,
  loading: props.loading,
  disabled: props.disabled,
  type: props.type,
  plain: props.variant === 'ghost' ? true : props.plain,
  rounded: props.rounded,
  outlined: props.variant === 'ghost' ? true : props.outlined,
  text: props.variant === 'link' ? true : props.text,
  title: props.title,
}))
</script>

<style scoped>
.fp-btn--has-content :deep(.p-button-label:empty) {
  display: none;
}
.fp-btn {
  white-space: nowrap;
  flex-shrink: 0;
  transition:
    transform 120ms var(--fp-ease-out),
    filter 120ms var(--fp-ease-out),
    opacity 120ms var(--fp-ease-out);
}
.fp-btn:not(:disabled):hover {
  filter: brightness(1.06);
}
.fp-btn:not(:disabled):active {
  transform: translateY(1px);
  filter: brightness(0.94);
}
@media (prefers-reduced-motion: reduce) {
  .fp-btn {
    transition: none;
  }
  .fp-btn:not(:disabled):active {
    transform: none;
  }
}
</style>
