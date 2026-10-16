import { defineConfig, presetWind4 } from 'unocss'

/**
 * 图标唯一来源：`@openvue/openicons`（字体图标，`oi-*` 类，随包本地分发）。
 *
 * T0-3（Doc/23 §3.1 / P0-4）：此处**刻意不启用** `presetIcons`。历史配置带
 * `cdn: 'https://esm.sh/'`，一旦源码出现 `i-*` 图标类，构建期就会去 esm.sh 抓
 * `@iconify-json/*`——内网/断外网环境下 `npm run build` 直接失败（实测
 * `FetchError: [GET] https://esm.sh/@iconify-json/mdi/icons.json`），且引入第三方
 * CDN 供应链风险。若将来确需图标集，改为安装 `@iconify-json/*` 本地包再启用，
 * 禁止重新挂任何远程 CDN（`iconSource.spec.ts` 已加回归护栏）。
 */
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
