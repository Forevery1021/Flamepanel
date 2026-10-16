import { definePreset } from '@openvue/themes'
import Aura from '@openvue/themes/aura'

/**
 * FlamePreset — FlamePanel 品牌主题预设
 * 基于 Aura，桥接设计令牌（--fp-*）到 OpenVue 组件（--p-*）。
 * 品牌色：全部取自 --fp-brand-* 运行时令牌（stores/theme.ts 写种子），暗色随 .dark 切换。
 */
/** 弹出式菜单 / 日期面板也引用 content.background——浮层必须保持不透明 */
const OPAQUE_OVERLAY = { background: 'var(--fp-bg-elevated)' }

export default definePreset(Aura, {
  components: {
    menu: { root: OPAQUE_OVERLAY },
    tieredmenu: { root: OPAQUE_OVERLAY },
    contextmenu: { root: OPAQUE_OVERLAY },
    datepicker: { panel: OPAQUE_OVERLAY },
    // 顶栏面包屑背景透明（否则半透明材质下像一块浮起的色块）
    breadcrumb: { root: { background: 'transparent' } },
  },
  semantic: {
    // 表单控件紧凑化（W4-a）：Aura 默认 0.5rem 内边距 + 1rem 字号 ≈ 42px 高，比 small 按钮（32px）
    // 高出一截，设置页等表单视觉「又大又松」。收到约 34px，与按钮基线对齐。
    formField: {
      paddingX: '0.75rem',
      paddingY: '0.4375rem',
      sm: { fontSize: '0.8125rem', paddingX: '0.625rem', paddingY: '0.3125rem' },
    },
    // 圆角桥接到运行时令牌：「设置 → 主题 → 圆角」同时作用于 OpenVue 组件（旧版写死 8/12/16）
    borderRadius: {
      md: 'var(--fp-radius-sm)',
      lg: 'var(--fp-radius-md)',
      xl: 'var(--fp-radius-lg)',
    },
    // 色板桥接到运行时品牌令牌：换预设 / 拖滑块时 OpenVue 组件（按钮、开关、选择器、
    // 分页、标签页……）与 --fp-* 同步换色。旧版写死火焰橙，是「颜色预设不生效」的根因。
    primary: {
      50: 'var(--fp-brand-50)',
      100: 'var(--fp-brand-100)',
      200: 'var(--fp-brand-200)',
      300: 'var(--fp-brand-300)',
      400: 'var(--fp-brand-400)',
      500: 'var(--fp-brand-500)',
      600: 'var(--fp-brand-600)',
      700: 'var(--fp-brand-700)',
      800: 'var(--fp-brand-800)',
      900: 'var(--fp-brand-900)',
      950: 'var(--fp-brand-900)',
    },
    colorScheme: {
      light: {
        primary: {
          color: 'var(--fp-brand)',
          contrastColor: 'var(--fp-brand-fg)',
          hoverColor: 'var(--fp-brand-hover)',
          activeColor: 'var(--fp-brand-active)',
        },
        highlight: {
          background: '{primary.50}',
          focusBackground: '{primary.100}',
          color: '{primary.800}',
          focusColor: '{primary.900}',
        },
        content: {
          // 内容表面随材质半透明（solid 下 = elevated）；浮层 / 表单仍保持不透明
          background: 'var(--fp-bg-surface)',
          hoverBackground: 'var(--fp-bg-hover)',
          borderColor: 'var(--fp-border)',
        },
        overlay: {
          select: {
            background: 'var(--fp-bg-elevated)',
            borderColor: 'var(--fp-border)',
            color: 'var(--fp-text-primary)',
          },
          popover: {
            background: 'var(--fp-bg-elevated)',
            borderColor: 'var(--fp-border)',
            color: 'var(--fp-text-primary)',
          },
          modal: {
            background: 'var(--fp-bg-elevated)',
            borderColor: 'var(--fp-border)',
            color: 'var(--fp-text-primary)',
          },
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
          floatLabelFocusColor: 'var(--fp-brand)',
        },
      },
      dark: {
        primary: {
          color: 'var(--fp-brand)',
          contrastColor: 'var(--fp-brand-fg)',
          hoverColor: 'var(--fp-brand-hover)',
          activeColor: 'var(--fp-brand-active)',
        },
        highlight: {
          background: 'var(--fp-brand-soft)',
          focusBackground: 'oklch(from var(--fp-brand) l c h / 0.24)',
          color: 'var(--fp-text-primary)',
          focusColor: 'var(--fp-text-primary)',
        },
        content: {
          // 内容表面随材质半透明（solid 下 = elevated）；浮层 / 表单仍保持不透明
          background: 'var(--fp-bg-surface)',
          hoverBackground: 'var(--fp-bg-hover)',
          borderColor: 'var(--fp-border)',
        },
        overlay: {
          select: {
            background: 'var(--fp-bg-elevated)',
            borderColor: 'var(--fp-border)',
            color: 'var(--fp-text-primary)',
          },
          popover: {
            background: 'var(--fp-bg-elevated)',
            borderColor: 'var(--fp-border)',
            color: 'var(--fp-text-primary)',
          },
          modal: {
            background: 'var(--fp-bg-elevated)',
            borderColor: 'var(--fp-border)',
            color: 'var(--fp-text-primary)',
          },
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
          floatLabelFocusColor: 'var(--fp-brand)',
        },
      },
    },
  },
})
