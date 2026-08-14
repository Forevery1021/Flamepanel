<template>
  <div class="settings-card">
    <h3 class="settings-section">{{ t('settingsTheme.preset') }}</h3>
    <div class="preset-grid">
      <button
        v-for="p in presets"
        :key="p.id"
        class="preset-card"
        :class="{ 'is-active': themeStore.preset === p.id }"
        @click="themeStore.setPreset(p.id)"
      >
        <span class="preset-swatch" :style="swatchStyle(p.id)" />
        <span class="preset-name">{{ t(p.labelKey) }}</span>
        <span class="preset-desc">{{ t(p.descKey) }}</span>
      </button>
    </div>

    <FpDivider />

    <h3 class="settings-section">{{ t('settingsTheme.brandColor') }}</h3>
    <div class="custom-controls">
      <div class="control-row">
        <label class="field-label">{{ t('settingsTheme.hue') }}</label>
        <FpSlider v-model="custom.hue" :min="0" :max="360" :step="1" class="control-slider" />
        <span class="mono control-value">{{ custom.hue }}°</span>
      </div>
      <div class="control-row">
        <label class="field-label">{{ t('settingsTheme.saturation') }}</label>
        <FpSlider v-model="custom.saturation" :min="0" :max="100" :step="1" class="control-slider" />
        <span class="mono control-value">{{ custom.saturation }}%</span>
      </div>
      <div class="control-row">
        <label class="field-label">{{ t('settingsTheme.lightness') }}</label>
        <FpSlider v-model="custom.lightness" :min="25" :max="80" :step="1" class="control-slider" />
        <span class="mono control-value">{{ custom.lightness }}%</span>
      </div>
      <div class="control-row">
        <label class="field-label">{{ t('settingsTheme.glassBlur') }}</label>
        <FpSlider v-model="custom.glassBlur" :min="0" :max="24" :step="1" class="control-slider" />
        <span class="mono control-value">{{ custom.glassBlur }}px</span>
      </div>
      <div class="control-row">
        <label class="field-label">{{ t('settingsTheme.glassEnabled') }}</label>
        <FpSwitch :model-value="themeStore.glassEnabled" @update:model-value="toggleGlass" />
        <span class="hint">{{ t('settingsTheme.glassEnabledHint') }}</span>
      </div>
      <div class="control-row">
        <label class="field-label">{{ t('settingsTheme.radius') }}</label>
        <FpSelectButton
          v-model="custom.radius"
          :options="radiusOptions"
          option-label="label"
          option-value="value"
        />
      </div>
      <div class="control-row">
        <label class="field-label">{{ t('settingsTheme.density') }}</label>
        <FpSelectButton
          v-model="custom.density"
          :options="densityOptions"
          option-label="label"
          option-value="value"
        />
      </div>
    </div>

    <div class="settings-actions">
      <FpButton variant="primary" icon="oi oi-download" @click="exportTheme">
        {{ t('settingsTheme.export') }}
      </FpButton>
      <FpButton variant="ghost" icon="oi oi-upload" @click="importTheme">
        {{ t('settingsTheme.import') }}
      </FpButton>
      <FpButton variant="ghost" icon="oi oi-refresh" @click="themeStore.resetCustom()">
        {{ t('settingsTheme.reset') }}
      </FpButton>
    </div>
    <input ref="importFileRef" type="file" accept="application/json" class="hidden-file" @change="onImportFile" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useThemeStore, PRESET_META, type ThemePreset } from '@/stores/theme'
import { useAppearanceStore } from '@/stores/appearance'
import { setLanguage, i18n } from '@/locales'
import { useFpToast } from '@/components/ui/FpToast'
import FpDivider from '@/components/ui/FpDivider.vue'
import FpSlider from '@/components/ui/FpSlider.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import FpSelectButton from '@/components/ui/FpSelectButton.vue'
import FpButton from '@/components/ui/FpButton.vue'

const { t } = useI18n()
const toast = useFpToast()
const themeStore = useThemeStore()
const appearance = useAppearanceStore()

const presets = (Object.keys(PRESET_META) as ThemePreset[]).map((id) => ({
  id,
  labelKey: PRESET_META[id].labelKey,
  descKey: PRESET_META[id].descKey,
}))

function swatchStyle(id: ThemePreset) {
  const map: Record<string, string> = {
    flame: 'linear-gradient(135deg, var(--fp-brand), var(--fp-brand-strong))',
    aurora: 'linear-gradient(135deg, var(--fp-warning), var(--fp-brand))',
    infinity: 'linear-gradient(135deg, var(--fp-warning), var(--fp-brand-strong))',
    custom: 'conic-gradient(from 0deg, oklch(0.6 0.2 0), oklch(0.6 0.2 60), oklch(0.6 0.2 120), oklch(0.6 0.2 180), oklch(0.6 0.2 240), oklch(0.6 0.2 300), oklch(0.6 0.2 360))',
  }
  return { background: map[id] ?? map.flame }
}

const custom = computed(() => themeStore.custom)
const radiusOptions = [
  { label: t('settingsTheme.radiusSharp'), value: 'sharp' },
  { label: t('settingsTheme.radiusStandard'), value: 'standard' },
  { label: t('settingsTheme.radiusRounded'), value: 'rounded' },
]
const densityOptions = [
  { label: t('settingsTheme.densityCompact'), value: 'compact' },
  { label: t('settingsTheme.densityStandard'), value: 'standard' },
  { label: t('settingsTheme.densityComfortable'), value: 'comfortable' },
]

function toggleGlass(next: boolean) {
  themeStore.setGlassEnabled(next)
}

function exportTheme() {
  const payload = {
    version: 2,
    exportedAt: new Date().toISOString(),
    theme: {
      mode: themeStore.mode,
      preset: themeStore.preset,
      glass: themeStore.glassEnabled,
      custom: themeStore.custom,
    },
    appearance: {
      menuTabs: appearance.state.menuTabs,
      menuAccordion: appearance.state.menuAccordion,
      hideMenu: appearance.state.hideMenu,
      menuCollapsed: appearance.state.menuCollapsed,
    },
    language: i18n.global.locale.value as string,
  }
  const blob = new Blob([JSON.stringify(payload, null, 2)], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = 'flamepanel-theme.json'
  a.click()
  URL.revokeObjectURL(url)
}

const importFileRef = ref<HTMLInputElement>()
function importTheme() {
  importFileRef.value?.click()
}
function onImportFile(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  const reader = new FileReader()
  reader.onload = () => {
    try {
      const data = JSON.parse(String(reader.result))
      const theme = data.theme ?? data
      if (theme.mode) themeStore.setMode(theme.mode === 'dark' ? 'dark' : 'light')
      if (theme.glass !== undefined) themeStore.setGlassEnabled(!!theme.glass)
      if (theme.preset) themeStore.setPreset(theme.preset)
      if (theme.custom) themeStore.updateCustom(theme.custom)
      if (data.appearance) {
        appearance.update({
          menuTabs: data.appearance.menuTabs ?? appearance.state.menuTabs,
          menuAccordion: data.appearance.menuAccordion ?? appearance.state.menuAccordion,
          hideMenu: Array.isArray(data.appearance.hideMenu)
            ? data.appearance.hideMenu
            : appearance.state.hideMenu,
          menuCollapsed: data.appearance.menuCollapsed ?? appearance.state.menuCollapsed,
        })
      }
      if (data.language) setLanguage(data.language)
      toast.success(t('common.success'))
    } catch {
      toast.error(t('common.failed'))
    }
  }
  reader.readAsText(file)
  input.value = ''
}
</script>

<style scoped>
.preset-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: var(--fp-space-3);
}
.preset-card {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 4px;
  padding: var(--fp-space-3);
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-elevated);
  cursor: pointer;
  transition: border-color 0.15s;
  text-align: left;
}
.preset-card:hover {
  border-color: var(--fp-brand);
}
.preset-card.is-active {
  border-color: var(--fp-brand);
  box-shadow: 0 0 0 1px var(--fp-brand);
}
.preset-swatch {
  width: 100%;
  height: 40px;
  border-radius: var(--fp-radius-sm);
}
.preset-name {
  font-size: 13px;
  font-weight: 500;
}
.preset-desc {
  font-size: 12px;
  color: var(--fp-text-muted);
}
.custom-controls {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
  max-width: 520px;
}
.control-row {
  display: flex;
  align-items: center;
  gap: var(--fp-space-3);
}
.field-label {
  font-size: 13px;
  color: var(--fp-text-secondary);
  min-width: 110px;
}
.control-slider {
  flex: 1;
}
.control-value {
  min-width: 56px;
}
.mono {
  font-family: var(--fp-font-mono);
  font-size: 12px;
}
.hint {
  font-size: 12px;
  color: var(--fp-text-muted);
}
.settings-actions {
  display: flex;
  gap: var(--fp-space-2);
  margin-top: var(--fp-space-4);
}
.hidden-file {
  display: none;
}
</style>
