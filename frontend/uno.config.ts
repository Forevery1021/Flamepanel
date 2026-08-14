import { defineConfig, presetWind4, presetIcons } from 'unocss'

export default defineConfig({
  presets: [
    presetWind4({
      theme: {
        colors: {
          brand: {
            DEFAULT: 'var(--fp-brand)',
            soft: 'var(--fp-brand-soft)',
            strong: 'var(--fp-brand-strong)',
          },
          surface: {
            app: 'var(--fp-bg-app)',
            elevated: 'var(--fp-bg-elevated)',
            hover: 'var(--fp-bg-hover)',
            border: 'var(--fp-border)',
          },
        },
        fontFamily: {
          sans: 'var(--fp-font-sans)',
          mono: 'var(--fp-font-mono)',
        },
        boxShadow: {
          xs: 'var(--fp-shadow-xs)',
          sm: 'var(--fp-shadow-sm)',
          md: 'var(--fp-shadow-md)',
          lg: 'var(--fp-shadow-lg)',
          brand: 'var(--fp-shadow-brand)',
        },
      },
    }),
    presetIcons({
      scale: 1.1,
      cdn: 'https://esm.sh/',
    }),
  ],
  shortcuts: {
    'glass': 'backdrop-blur-sm bg-white/60 dark:bg-zinc-900/55 border border-white/40 dark:border-white/10',
    'glass-heavy': 'backdrop-blur-2xl bg-white/75 dark:bg-zinc-900/75 border border-white/40 dark:border-white/10',
    'card-panel': 'bg-surface-elevated border border-surface-border rounded-lg shadow-sm',
    // 通用卡片面板（原 17 处视图 scoped 重复的 .panel 样式）
    'panel': 'p-[var(--fp-space-4)] rounded-[var(--fp-radius-md)] bg-[var(--fp-bg-elevated)] border border-[var(--fp-border)]',
    'panel-header': 'flex items-center justify-between mb-[var(--fp-space-3)]',
    'panel-title': 'text-[14.5px] font-semibold text-[var(--fp-text-primary)]',
    'info-row': 'flex items-center justify-between py-[var(--fp-space-1)] text-[13px]',
    'row-actions': 'flex items-center gap-[6px]',
    'z-header': 'z-[var(--fp-z-header)]',
    'z-sidebar': 'z-[var(--fp-z-sidebar)]',
    'z-overlay': 'z-[var(--fp-z-overlay)]',
    'z-modal': 'z-[var(--fp-z-modal)]',
    'z-toast': 'z-[var(--fp-z-toast)]',
  },
  content: {
    pipeline: {
      include: [/\.(vue|ts|tsx|js|jsx|md)($|\?)/],
    },
  },
})
