<template>
  <div class="settings-group fp-surface">
    <SettingRow :label="t('settings.version')">
      <code class="settings-code">v{{ health?.version ?? '—' }}</code>
    </SettingRow>
    <SettingRow :label="t('settings.uptime')">
      <span class="settings-value">{{ uptimeText }}</span>
    </SettingRow>
    <SettingRow :label="t('settings.aboutCli')" :description="t('settings.aboutCliHint')">
      <code class="settings-code">fpctl status</code>
    </SettingRow>
    <SettingRow :label="t('settings.aboutApi')" :description="t('settings.aboutApiHint')">
      <FpButton variant="secondary" outlined icon="oi oi-book" @click="router.push('/api-docs')">
        {{ t('settings.aboutApiOpen') }}
      </FpButton>
    </SettingRow>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'

import type { HealthDetail } from '@/api/generated'
import { fetchHealthDetail } from '@/api/health'
import FpButton from '@/components/ui/FpButton.vue'
import SettingRow from '@/components/settings/SettingRow.vue'

/** 面板设置 · 关于：版本、运行时长、CLI / API 文档入口。进入分区时加载一次健康详情。 */
const { t } = useI18n()
const router = useRouter()

const health = ref<HealthDetail | null>(null)
const uptimeText = computed(() => {
  const s = health.value?.uptime_secs
  if (s == null) return '—'
  const d = Math.floor(s / 86400)
  const hh = Math.floor((s % 86400) / 3600)
  const mm = Math.floor((s % 3600) / 60)
  return t('settings.uptimeValue', { d, h: hh, m: mm })
})

onMounted(async () => {
  try {
    health.value = (await fetchHealthDetail()).data
  } catch {
    health.value = null
  }
})
</script>

<style scoped>
.settings-code {
  padding: 2px 8px;
  border-radius: var(--fp-radius-sm);
  background: var(--fp-bg-subtle);
  border: 1px solid var(--fp-border);
  font-family: var(--fp-font-mono);
  font-size: var(--fp-font-xs);
  color: var(--fp-text-primary);
}
.settings-value {
  font-size: var(--fp-font-sm);
  color: var(--fp-text-primary);
}
</style>
