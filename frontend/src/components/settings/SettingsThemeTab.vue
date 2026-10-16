<template>
  <div class="theme-tab">
    <!-- ── 颜色 ── -->
    <section class="settings-group fp-surface">
      <h3 class="settings-group__title">{{ t('settingsTheme.groupColor') }}</h3>
      <SettingRow :label="t('settingsTheme.mode')" :description="t('settingsTheme.modeHint')">
        <FpSelectButton
          :model-value="themeStore.mode"
          :options="modeOptions"
          option-label="label"
          option-value="value"
          :allow-empty="false"
          size="small"
          :aria-label="t('settingsTheme.mode')"
          @update:model-value="(v) => themeStore.setMode(v as ThemeMode)"
        />
      </SettingRow>

      <SettingRow :label="t('settingsTheme.preset')" :description="currentPresetDesc">
        <div class="swatches" role="radiogroup" :aria-label="t('settingsTheme.preset')">
          <button
            v-for="p in BUILTIN_PRESETS"
            :key="p"
            type="button"
            role="radio"
            class="swatch"
            :class="{ 'is-active': themeStore.preset === p }"
            :style="{ '--swatch': presetSwatch(p), '--swatch-color': presetColor(p) }"
            :aria-checked="themeStore.preset === p"
            :aria-label="t(PRESET_META[p].labelKey)"
            :title="t(PRESET_META[p].labelKey)"
            @click="themeStore.setPreset(p)"
          >
            <i v-if="themeStore.preset === p" class="oi oi-check" aria-hidden="true" />
          </button>
          <button
            type="button"
            role="radio"
            class="swatch swatch--custom"
            :class="{ 'is-active': themeStore.preset === 'custom' }"
            :style="{ '--swatch': customSwatch, '--swatch-color': customSwatch }"
            :aria-checked="themeStore.preset === 'custom'"
            :aria-label="t('settingsTheme.custom')"
            :title="t('settingsTheme.custom')"
            @click="themeStore.setPreset('custom')"
          >
            <i v-if="themeStore.preset === 'custom'" class="oi oi-check" aria-hidden="true" />
          </button>
        </div>
      </SettingRow>

      <SettingRow
        :label="t('settingsTheme.brandColor')"
        :description="t('settingsTheme.brandHint')"
      >
        <div class="sliders">
          <label class="slider-row">
            <span class="slider-row__label">{{ t('settingsTheme.hue') }}</span>
            <FpSlider
              class="slider-row__track slider-row__track--hue"
              :model-value="themeStore.seed.hue"
              :min="0"
              :max="360"
              :aria-label="t('settingsTheme.hue')"
              @update:model-value="(v) => themeStore.tweakBrand({ hue: v })"
            />
            <span class="slider-row__value">{{ themeStore.seed.hue }}°</span>
          </label>
          <label class="slider-row">
            <span class="slider-row__label">{{ t('settingsTheme.saturation') }}</span>
            <FpSlider
              class="slider-row__track"
              :model-value="themeStore.seed.saturation"
              :min="0"
              :max="100"
              :aria-label="t('settingsTheme.saturation')"
              @update:model-value="(v) => themeStore.tweakBrand({ saturation: v })"
            />
            <span class="slider-row__value">{{ themeStore.seed.saturation }}%</span>
          </label>
          <label class="slider-row">
            <span class="slider-row__label">{{ t('settingsTheme.lightness') }}</span>
            <FpSlider
              class="slider-row__track"
              :model-value="themeStore.seed.lightness"
              :min="25"
              :max="85"
              :aria-label="t('settingsTheme.lightness')"
              @update:model-value="(v) => themeStore.tweakBrand({ lightness: v })"
            />
            <span class="slider-row__value">{{ themeStore.seed.lightness }}%</span>
          </label>
        </div>
      </SettingRow>
    </section>

    <!-- ── 材质 ── -->
    <section class="settings-group fp-surface">
      <h3 class="settings-group__title">{{ t('settingsTheme.material') }}</h3>
      <SettingRow :label="t('settingsTheme.material')" :description="currentMaterialHint">
        <div class="materials" role="radiogroup" :aria-label="t('settingsTheme.material')">
          <button
            v-for="m in materialOptions"
            :key="m.value"
            type="button"
            role="radio"
            class="material"
            :class="[`material--${m.value}`, { 'is-active': themeStore.material === m.value }]"
            :aria-checked="themeStore.material === m.value"
            @click="themeStore.setMaterial(m.value)"
          >
            <span class="material__preview" aria-hidden="true">
              <span class="material__blob" />
              <span class="material__pane" />
            </span>
            <span class="material__label">{{ m.label }}</span>
          </button>
        </div>
      </SettingRow>
      <SettingRow
        :label="t('settingsTheme.glassBlur')"
        :description="t('settingsTheme.glassBlurHint')"
      >
        <label class="slider-row slider-row--single">
          <FpSlider
            class="slider-row__track"
            :model-value="themeStore.custom.glassBlur"
            :min="0"
            :max="40"
            :disabled="themeStore.material === 'solid'"
            :aria-label="t('settingsTheme.glassBlur')"
            @update:model-value="(v) => themeStore.updateCustom({ glassBlur: v })"
          />
          <span class="slider-row__value">{{ themeStore.custom.glassBlur }}px</span>
        </label>
      </SettingRow>
    </section>

    <!-- ── 形状与密度 ── -->
    <section class="settings-group fp-surface">
      <h3 class="settings-group__title">{{ t('settingsTheme.groupShape') }}</h3>
      <SettingRow :label="t('settingsTheme.radius')">
        <FpSelectButton
          :model-value="themeStore.custom.radius"
          :options="radiusOptions"
          option-label="label"
          option-value="value"
          :allow-empty="false"
          size="small"
          :aria-label="t('settingsTheme.radius')"
          @update:model-value="(v) => themeStore.updateCustom({ radius: v as RadiusScale })"
        />
      </SettingRow>
      <SettingRow :label="t('settingsTheme.density')" :description="t('settingsTheme.densityHint')">
        <FpSelectButton
          :model-value="themeStore.custom.density"
          :options="densityOptions"
          option-label="label"
          option-value="value"
          :allow-empty="false"
          size="small"
          :aria-label="t('settingsTheme.density')"
          @update:model-value="(v) => themeStore.updateCustom({ density: v as DensityScale })"
        />
      </SettingRow>
      <SettingRow :label="t('settingsTheme.preview')" :description="t('settingsTheme.previewHint')">
        <div class="preview fp-surface" aria-hidden="true">
          <FpButton size="small" tabindex="-1">{{ t('settingsTheme.previewPrimary') }}</FpButton>
          <FpButton size="small" variant="secondary" outlined tabindex="-1">
            {{ t('common.cancel') }}
          </FpButton>
          <FpTag severity="success" :value="t('settingsTheme.previewTag')" />
          <span class="preview__link">{{ t('settingsTheme.previewLink') }}</span>
        </div>
      </SettingRow>
    </section>

    <div class="theme-tab__actions">
      <FpButton size="small" variant="ghost" text icon="oi oi-upload" @click="importTheme">
        {{ t('settingsTheme.import') }}
      </FpButton>
      <FpButton size="small" variant="ghost" text icon="oi oi-download" @click="exportTheme">
        {{ t('settingsTheme.export') }}
      </FpButton>
      <FpButton size="small" variant="ghost" text icon="oi oi-refresh" @click="resetTheme">
        {{ t('settingsTheme.reset') }}
      </FpButton>
      <input
        ref="importFileRef"
        type="file"
        accept="application/json,.json"
        class="sr-only"
        tabindex="-1"
        @change="onImportFile"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  useThemeStore,
  BUILTIN_PRESETS,
  PRESET_META,
  THEME_MATERIALS,
  THEME_MODES,
  presetColor,
  presetSwatch,
  seedToOklch,
  type ThemeMaterial,
  type ThemeMode,
  type ThemePreset,
  type RadiusScale,
  type DensityScale,
} from '@/stores/theme'
import { useAppearanceStore } from '@/stores/appearance'
import { setLanguage, i18n } from '@/locales'
import { useFpToast } from '@/components/ui/FpToast'
import FpSlider from '@/components/ui/FpSlider.vue'
import FpSelectButton from '@/components/ui/FpSelectButton.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import SettingRow from './SettingRow.vue'

/**
 * 主题定制（W4-a 重写）。
 *
 * 旧版的三个问题：预设卡片是写死的渐变（与实际颜色无关、三张几乎一样）；滑块只改 `custom`
 * 而当前预设不是 custom 时根本不读它；材质切换只作用于顶栏。现在：
 * - 色块直接用预设种子色渲染，点击即换色；
 * - 滑块走 `tweakBrand`：非自定义预设下以当前色为起点自动切到「自定义」，拖动立即可见；
 * - 材质作用于全部内容表面（`--fp-bg-surface`），并有环境光斑垫底，切换差异肉眼可见。
 * 所有偏好都是本地即时生效，无需「保存」。
 */
const { t } = useI18n()
const toast = useFpToast()
const themeStore = useThemeStore()
const appearance = useAppearanceStore()

const modeOptions = computed(() =>
  THEME_MODES.map((value) => ({ value, label: t(`settingsTheme.modes.${value}`) })),
)
const radiusOptions = computed(() => [
  { label: t('settingsTheme.radiusSharp'), value: 'sharp' },
  { label: t('settingsTheme.radiusStandard'), value: 'standard' },
  { label: t('settingsTheme.radiusRounded'), value: 'rounded' },
])
const densityOptions = computed(() => [
  { label: t('settingsTheme.densityCompact'), value: 'compact' },
  { label: t('settingsTheme.densityStandard'), value: 'standard' },
  { label: t('settingsTheme.densityComfortable'), value: 'comfortable' },
])
const materialOptions = computed<Array<{ value: ThemeMaterial; label: string; hint: string }>>(() =>
  THEME_MATERIALS.map((value) => ({
    value,
    label: t(`settingsTheme.materials.${value}`),
    hint: t(`settingsTheme.materials.${value}Hint`),
  })),
)

const currentPresetDesc = computed(() => {
  const meta = PRESET_META[themeStore.preset as ThemePreset] ?? PRESET_META.flame
  return `${t(meta.labelKey)} · ${t(meta.descKey)}`
})
const currentMaterialHint = computed(
  () => materialOptions.value.find((m) => m.value === themeStore.material)?.hint ?? '',
)
const customSwatch = computed(() => {
  const { l, c, h } = seedToOklch(themeStore.custom, false)
  return `oklch(${l} ${c} ${h})`
})

function resetTheme() {
  themeStore.resetCustom()
  themeStore.setMode('dark')
  toast.success(t('common.success'))
}

function exportTheme() {
  const payload = {
    version: 3,
    exportedAt: new Date().toISOString(),
    theme: {
      mode: themeStore.mode,
      preset: themeStore.preset,
      material: themeStore.material,
      custom: { ...themeStore.custom, appBackground: '', loginBackground: '' },
    },
    appearance: {
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

/** 导入：只接受已知字段；未知 / 越界值由 store 的 setter 丢弃（不会写进脏状态） */
function applyImported(data: Record<string, unknown>) {
  const theme = (data.theme ?? data) as Record<string, unknown>
  if (typeof theme.mode === 'string') themeStore.setMode(theme.mode as ThemeMode)
  if (typeof theme.material === 'string') themeStore.setMaterial(theme.material as ThemeMaterial)
  else if (typeof theme.glass === 'boolean') themeStore.setGlassEnabled(theme.glass)
  if (theme.custom && typeof theme.custom === 'object') {
    const c = theme.custom as Record<string, unknown>
    const patch: Record<string, unknown> = {}
    for (const k of ['hue', 'saturation', 'lightness', 'glassBlur'] as const) {
      if (typeof c[k] === 'number' && Number.isFinite(c[k])) patch[k] = c[k]
    }
    if (typeof c.radius === 'string') patch.radius = c.radius
    if (typeof c.density === 'string') patch.density = c.density
    themeStore.updateCustom(patch)
  }
  if (typeof theme.preset === 'string') themeStore.setPreset(theme.preset as ThemePreset)
  const ap = data.appearance as Record<string, unknown> | undefined
  if (ap && typeof ap === 'object') {
    appearance.update({
      menuAccordion:
        typeof ap.menuAccordion === 'boolean' ? ap.menuAccordion : appearance.state.menuAccordion,
      hideMenu: Array.isArray(ap.hideMenu) ? (ap.hideMenu as string[]) : appearance.state.hideMenu,
      menuCollapsed:
        typeof ap.menuCollapsed === 'boolean' ? ap.menuCollapsed : appearance.state.menuCollapsed,
    })
  }
  if (typeof data.language === 'string') setLanguage(data.language)
}

function onImportFile(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  const reader = new FileReader()
  reader.onload = () => {
    try {
      applyImported(JSON.parse(String(reader.result)))
      toast.success(t('common.success'))
    } catch {
      toast.warning(t('settingsTheme.importInvalid'))
    }
  }
  reader.readAsText(file)
  input.value = ''
}

defineExpose({ applyImported })
</script>

<style scoped>
.theme-tab {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-4);
}

/* 强调色：圆形色块（24px），选中带外圈 + 对勾 */
.swatches {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 10px;
}
.swatch {
  display: inline-grid;
  place-items: center;
  width: 26px;
  height: 26px;
  padding: 0;
  border: 0;
  border-radius: 999px;
  background: var(--swatch);
  color: oklch(1 0 0);
  font-size: 12px;
  cursor: pointer;
  box-shadow: inset 0 0 0 1px oklch(0 0 0 / 0.12);
  transition:
    transform 120ms var(--fp-ease-out),
    box-shadow 120ms var(--fp-ease-out);
}
.swatch:hover {
  transform: scale(1.08);
}
.swatch.is-active {
  box-shadow:
    0 0 0 2px var(--fp-bg-elevated),
    0 0 0 4px var(--swatch-color);
}
.swatch--custom {
  background:
    radial-gradient(circle, var(--swatch) 0 45%, transparent 47%),
    conic-gradient(
      oklch(0.7 0.18 0),
      oklch(0.7 0.18 90),
      oklch(0.7 0.18 180),
      oklch(0.7 0.18 270),
      oklch(0.7 0.18 360)
    );
}

/* 品牌色滑块 */
.sliders {
  display: flex;
  flex-direction: column;
  gap: 10px;
  width: min(100%, 360px);
}
.slider-row {
  display: grid;
  grid-template-columns: 52px minmax(0, 1fr) 44px;
  align-items: center;
  gap: var(--fp-space-3);
  font-size: var(--fp-font-xs);
  color: var(--fp-text-secondary);
}
.slider-row--single {
  grid-template-columns: minmax(0, 1fr) 44px;
  width: min(100%, 360px);
}
.slider-row__value {
  text-align: right;
  font-family: var(--fp-font-mono);
  font-variant-numeric: tabular-nums;
  color: var(--fp-text-primary);
}
.slider-row__track--hue :deep(.p-slider) {
  background: linear-gradient(
    90deg,
    oklch(0.7 0.16 0),
    oklch(0.7 0.16 60),
    oklch(0.7 0.16 120),
    oklch(0.7 0.16 180),
    oklch(0.7 0.16 240),
    oklch(0.7 0.16 300),
    oklch(0.7 0.16 360)
  );
}
.slider-row__track--hue :deep(.p-slider-range) {
  background: transparent;
}

/* 材质：小尺寸示意卡（80×52），比旧版大卡片更紧凑 */
.materials {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: var(--fp-space-2);
}
.material {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 6px;
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-md);
  background: transparent;
  color: var(--fp-text-secondary);
  font-size: var(--fp-font-xs);
  cursor: pointer;
}
.material:hover {
  border-color: var(--fp-border-strong);
}
.material.is-active {
  border-color: var(--fp-brand);
  color: var(--fp-text-primary);
  box-shadow: 0 0 0 3px var(--fp-brand-soft);
}
.material__preview {
  position: relative;
  width: 76px;
  height: 46px;
  overflow: hidden;
  border-radius: var(--fp-radius-sm);
  background: linear-gradient(135deg, var(--fp-brand-300), var(--fp-brand-700));
}
.material__blob {
  position: absolute;
  left: 8px;
  top: 6px;
  width: 22px;
  height: 22px;
  border-radius: 999px;
  background: oklch(0.85 0.14 calc(var(--fp-brand-h) + 150));
}
.material__pane {
  position: absolute;
  inset: 14px 6px 6px 22px;
  border-radius: 4px;
  border: 1px solid oklch(1 0 0 / 0.35);
}
.material--solid .material__pane {
  background: var(--fp-bg-elevated);
}
.material--glass .material__pane {
  background: oklch(1 0 0 / 0.35);
  backdrop-filter: blur(4px);
}
.material--acrylic .material__pane {
  background: oklch(1 0 0 / 0.5);
  backdrop-filter: blur(10px) saturate(1.6);
}

.preview {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: flex-end;
  gap: var(--fp-space-2);
  padding: var(--fp-space-3);
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-md);
  pointer-events: none;
}
.preview__link {
  font-size: var(--fp-font-sm);
  color: var(--fp-brand);
}

.theme-tab__actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: var(--fp-space-1);
}

@container settings (max-width: 560px) {
  .swatches,
  .materials,
  .preview {
    justify-content: flex-start;
  }
  .sliders,
  .slider-row--single {
    width: 100%;
  }
}
</style>
