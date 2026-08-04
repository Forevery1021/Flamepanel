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
  },
  content: {
    pipeline: {
      include: [/\.(vue|ts|tsx|js|jsx|md)($|\?)/],
    },
  },
})
