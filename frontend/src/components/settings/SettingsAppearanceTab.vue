<template>
  <div class="settings-group fp-surface">
    <h3 class="settings-group__title">{{ t('settingsAppearance.interface') }}</h3>
    <SettingRow
      v-slot="{ id }"
      :label="t('settingsAppearance.menuAccordion')"
      :description="t('settingsAppearance.menuAccordionHint')"
    >
      <FpSwitch
        :input-id="id"
        :model-value="appearance.state.menuAccordion"
        @update:model-value="toggleMenuAccordion"
      />
    </SettingRow>
    <SettingRow
      v-slot="{ id }"
      :label="t('settingsAppearance.menuShow')"
      :description="t('settingsAppearance.menuShowHint')"
    >
      <FpMultiSelect
        :input-id="id"
        :model-value="appearance.state.hideMenu"
        :options="menuGroupOptions"
        option-label="label"
        option-value="value"
        display="chip"
        :placeholder="t('settingsAppearance.menuShowNone')"
        @update:model-value="(v) => saveMenuVisibility(v ?? [])"
      />
    </SettingRow>
    <SettingRow
      :label="t('onboarding.replay')"
      :description="t('settingsAppearance.onboardingHint')"
    >
      <FpButton variant="secondary" outlined icon="oi oi-compass" @click="onboarding.start()">
        {{ t('settingsAppearance.replayShort') }}
      </FpButton>
    </SettingRow>
  </div>

  <div class="settings-group fp-surface">
    <h3 class="settings-group__title">{{ t('settingsAppearance.background') }}</h3>
    <SettingRow
      v-for="bg in backgrounds"
      :key="bg.target"
      :label="bg.label"
      :description="t('settingsAppearance.bgHint')"
    >
      <span
        v-if="bg.value"
        class="bg-preview"
        :style="{ backgroundImage: `url(${bg.value})` }"
        aria-hidden="true"
      />
      <FpButton variant="secondary" outlined icon="oi oi-upload" @click="pickBg(bg.target)">
        {{ t('settingsAppearance.upload') }}
      </FpButton>
      <FpButton v-if="bg.value" variant="ghost" text @click="clearBg(bg.target)">
        {{ t('settingsAppearance.clear') }}
      </FpButton>
    </SettingRow>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { updateSetting } from '@/api/settings'
import { useThemeStore } from '@/stores/theme'
import { useAppearanceStore } from '@/stores/appearance'
import { useOnboardingStore } from '@/stores/onboarding'
import FpButton from '@/components/ui/FpButton.vue'
import FpMultiSelect from '@/components/ui/FpMultiSelect.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import { useFpToast } from '@/components/ui/FpToast'
import SettingRow from '@/components/settings/SettingRow.vue'

/**
 * 面板设置 · 界面：菜单手风琴 / 菜单分组可见性 / 引导重放 / 背景图。
 * 全部「即时保存」（单键 `PUT /api/settings/{key}`），不参与表单分区的批量保存与脏状态。
 */
const { t } = useI18n()
const toast = useFpToast()
const themeStore = useThemeStore()
const appearance = useAppearanceStore()
const onboarding = useOnboardingStore()

const menuGroupOptions = computed(() => [
  { label: t('nav.groupWeb'), value: 'web' },
  { label: t('nav.groupApps'), value: 'apps' },
  { label: t('nav.groupStorage'), value: 'storage' },
  { label: t('nav.groupOps'), value: 'ops' },
  { label: t('nav.groupSystem'), value: 'system' },
])
async function persist(key: string, value: string) {
  try {
    await updateSetting(key, value)
    toast.success(t('common.success'))
  } catch (e: unknown) {
    toast.error(e, t('common.failed'))
  }
}

function toggleMenuAccordion(value: boolean) {
  appearance.update({ menuAccordion: value })
  persist('menu_accordion', value ? 'true' : 'false')
}

function saveMenuVisibility(hidden: unknown) {
  const hide = Array.isArray(hidden) ? hidden.map(String) : []
  appearance.update({ hideMenu: hide })
  persist('hide_menu', JSON.stringify(hide))
}

type BgTarget = 'app' | 'login'
const backgrounds = computed(() => [
  {
    target: 'app' as BgTarget,
    label: t('settingsAppearance.appBackground'),
    value: themeStore.custom.appBackground,
  },
  {
    target: 'login' as BgTarget,
    label: t('settingsAppearance.loginBackground'),
    value: themeStore.custom.loginBackground,
  },
])
const MAX_BG_BYTES = 500 * 1024

function pickBg(target: BgTarget) {
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = 'image/*'
  input.onchange = async () => {
    const file = input.files?.[0]
    if (!file) return
    try {
      applyBackground(target, await compressImage(file))
    } catch {
      toast.warning(t('settingsAppearance.tooLarge'))
    }
  }
  input.click()
}

function compressImage(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    if (file.size > 5 * 1024 * 1024) return reject(new Error('too large'))
    const img = new Image()
    const objectUrl = URL.createObjectURL(file)
    img.onload = () => {
      URL.revokeObjectURL(objectUrl)
      const scale = Math.min(1, 1920 / img.width)
      const canvas = document.createElement('canvas')
      canvas.width = Math.round(img.width * scale)
      canvas.height = Math.round(img.height * scale)
      const ctx = canvas.getContext('2d')
      if (!ctx) return reject(new Error('canvas'))
      ctx.drawImage(img, 0, 0, canvas.width, canvas.height)
      let quality = 0.85
      let dataUrl = canvas.toDataURL('image/jpeg', quality)
      while (dataUrl.length > MAX_BG_BYTES && quality > 0.4) {
        quality -= 0.1
        dataUrl = canvas.toDataURL('image/jpeg', quality)
      }
      if (dataUrl.length > MAX_BG_BYTES) return reject(new Error('too large'))
      resolve(dataUrl)
    }
    img.onerror = () => {
      URL.revokeObjectURL(objectUrl)
      reject(new Error('image'))
    }
    img.src = objectUrl
  })
}

function applyBackground(target: BgTarget, dataUrl: string) {
  themeStore.updateCustom(
    target === 'app' ? { appBackground: dataUrl } : { loginBackground: dataUrl },
  )
  persist(target === 'app' ? 'app_background' : 'login_background', dataUrl)
}

function clearBg(target: BgTarget) {
  themeStore.updateCustom(target === 'app' ? { appBackground: '' } : { loginBackground: '' })
  persist(target === 'app' ? 'app_background' : 'login_background', '')
}
</script>

<style scoped>
.bg-preview {
  width: 56px;
  height: 32px;
  border-radius: var(--fp-radius-sm);
  border: 1px solid var(--fp-border);
  background-size: cover;
  background-position: center;
}
</style>
