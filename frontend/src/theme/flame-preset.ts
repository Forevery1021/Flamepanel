import { definePreset } from '@openvue/themes'
import Aura from '@openvue/themes/aura'

/**
 * FlamePreset — FlamePanel 品牌主题预设
 * 基于 Aura，桥接设计令牌（--fp-*）到 OpenVue 组件（--p-*）。
 * 品牌色：火焰橙，暗色模式随 .dark 类切换。
 */
export default definePreset(Aura, {
  semantic: {
    borderRadius: {
      md: '8px',
      lg: '12px',
      xl: '16px',
    },
    primary: {
      50: '#fff7ed',
      100: '#ffedd5',
      200: '#fed7aa',
      300: '#fdba74',
      400: '#fb923c',
      500: '#f97316',
      600: '#ea580c',
      700: '#c2410c',
      800: '#9a3412',
      900: '#7c2d12',
      950: '#431407',
    },
    colorScheme: {
      light: {
        primary: {
          color: '{primary.600}',
          contrastColor: '#ffffff',
          hoverColor: '{primary.700}',
          activeColor: '{primary.800}',
        },
        highlight: {
          background: '{primary.50}',
          focusBackground: '{primary.100}',
          color: '{primary.800}',
          focusColor: '{primary.900}',
        },
        content: {
          background: 'var(--fp-bg-elevated)',
          hoverBackground: 'var(--fp-bg-hover)',
          borderColor: 'var(--fp-border)',
        },
        overlay: {
          select: { background: 'var(--fp-bg-elevated)', borderColor: 'var(--fp-border)', color: 'var(--fp-text-primary)' },
          popover: { background: 'var(--fp-bg-elevated)', borderColor: 'var(--fp-border)', color: 'var(--fp-text-primary)' },
          modal: { background: 'var(--fp-bg-elevated)', borderColor: 'var(--fp-border)', color: 'var(--fp-text-primary)' },
        },
        text: {
          color: 'var(--fp-text-primary)',
          hoverColor: 'var(--fp-text-primary)',
          mutedColor: 'var(--fp-text-secondary)',
          hoverMutedColor: 'var(--fp-text-secondary)',
        },
        formField: {
          background: 'var(--fp-bg-elevated)',
          filledBackground: 'var(--fp-bg-hover)',
          borderColor: 'var(--fp-border-strong)',
          hoverBorderColor: 'var(--fp-text-muted)',
          color: 'var(--fp-text-primary)',
          placeholderColor: 'var(--fp-text-muted)',
          floatLabelColor: 'var(--fp-text-secondary)',
          floatLabelFocusColor: '{primary.600}',
        },
      },
      dark: {
        primary: {
          color: '{primary.400}',
          contrastColor: '#ffffff',
          hoverColor: '{primary.300}',
          activeColor: '{primary.200}',
        },
        highlight: {
          background: 'color-mix(in srgb, {primary.400}, transparent 86%)',
          focusBackground: 'color-mix(in srgb, {primary.400}, transparent 78%)',
          color: '#ffffff',
          focusColor: '#ffffff',
        },
        content: {
          background: 'var(--fp-bg-elevated)',
          hoverBackground: 'var(--fp-bg-hover)',
          borderColor: 'var(--fp-border)',
        },
        overlay: {
          select: { background: 'var(--fp-bg-elevated)', borderColor: 'var(--fp-border)', color: 'var(--fp-text-primary)' },
          popover: { background: 'var(--fp-bg-elevated)', borderColor: 'var(--fp-border)', color: 'var(--fp-text-primary)' },
          modal: { background: 'var(--fp-bg-elevated)', borderColor: 'var(--fp-border)', color: 'var(--fp-text-primary)' },
        },
        text: {
          color: 'var(--fp-text-primary)',
          hoverColor: 'var(--fp-text-primary)',
          mutedColor: 'var(--fp-text-secondary)',
          hoverMutedColor: 'var(--fp-text-secondary)',
        },
        formField: {
          background: 'var(--fp-bg-elevated)',
          filledBackground: 'var(--fp-bg-hover)',
          borderColor: 'var(--fp-border-strong)',
          hoverBorderColor: 'var(--fp-text-muted)',
          color: 'var(--fp-text-primary)',
          placeholderColor: 'var(--fp-text-muted)',
          floatLabelColor: 'var(--fp-text-secondary)',
          floatLabelFocusColor: '{primary.400}',
        },
      },
    },
  },
})
