<template>
  <Button v-bind="btnAttrs" class="fp-btn">
    <slot />
  </Button>
</template>

<script setup lang="ts">
import { computed } from 'vue'
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

const btnAttrs = computed(() => ({
  severity: severityMap[props.variant],
  label: props.label,
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
.fp-btn {
  transition:
    transform 120ms var(--fp-ease-out),
    opacity 120ms var(--fp-ease-out);
}
.fp-btn:not(:disabled):active {
  transform: scale(0.97);
}
</style>
