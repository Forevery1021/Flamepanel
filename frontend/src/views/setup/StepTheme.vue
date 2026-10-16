<template>
  <div class="step">
    <h2 class="step__title">{{ t('setup.themeTitle') }}</h2>
    <p class="step__sub">{{ t('setup.themeSub') }}</p>

    <div class="step-themes">
      <label v-for="p in presets" :key="p" class="theme-card" :class="{ selected: theme === p }">
        <input
          type="radio"
          :value="p"
          :checked="theme === p"
          @change="emit('update', { theme: p })"
        />
        <span class="theme-card__swatch" :style="swatchStyle(p)" />
        <span class="theme-card__name">{{ t(`settingsTheme.${p}`) }}</span>
      </label>
    </div>

    <div class="step-lang">
      <span class="step-lang__label">{{ t('setup.language') }}</span>
      <FpRadioGroup
        :model-value="language"
        :options="langOptions"
        option-label="label"
        option-value="value"
        @update:model-value="emit('update', { language: $event as AppLocale })"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import FpRadioGroup from '@/components/ui/FpRadioGroup.vue'
import { BUILTIN_PRESETS, presetSwatch, type ThemePreset } from '@/stores/theme'
import type { AppLocale } from '@/locales'

defineProps<{
  theme: ThemePreset
  language: AppLocale
}>()

const emit = defineEmits<{
  update: [patch: { theme?: ThemePreset; language?: AppLocale }]
}>()

const { t } = useI18n()
const presets: ThemePreset[] = [...BUILTIN_PRESETS]
const langOptions = [
  { value: 'zh-CN', label: '简体中文' },
  { value: 'en-US', label: 'English' },
  { value: 'ja-JP', label: '日本語' },
]

/** 色块直接取预设种子色（与设置页同源），不再手写与实际主题不符的渐变 */
function swatchStyle(p: ThemePreset): Record<string, string> {
  return { background: presetSwatch(p === 'custom' ? 'flame' : p) }
}
</script>

<style scoped>
.step {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
}
.step__title {
  font-size: 20px;
  font-weight: 700;
  color: var(--fp-text-primary);
}
.step__sub {
  font-size: 13px;
  color: var(--fp-text-secondary);
}
.step-themes {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(84px, 1fr));
  gap: var(--fp-space-3);
}
.theme-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--fp-space-2);
  padding: var(--fp-space-4) var(--fp-space-2);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
  cursor: pointer;
  transition: border-color 0.15s;
}
.theme-card:hover {
  border-color: var(--fp-brand);
}
.theme-card.selected {
  border-color: var(--fp-brand);
  background: var(--fp-brand-soft);
}
.theme-card input {
  display: none;
}
.theme-card__swatch {
  width: 44px;
  height: 44px;
  border-radius: 50%;
  border: 2px solid rgba(255, 255, 255, 0.25);
}
.theme-card__name {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--fp-text-primary);
}
.step-lang {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-2);
}
.step-lang__label {
  font-size: 13px;
  font-weight: 600;
  color: var(--fp-text-primary);
}

@media (max-width: 480px) {
  .step-themes {
    grid-template-columns: repeat(2, 1fr);
  }
}
</style>
