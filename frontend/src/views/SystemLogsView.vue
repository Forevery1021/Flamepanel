<template>
  <LayoutContent :title="t('log.title')" :description="t('pageDesc.systemLogs')">
    <div class="panel">
      <FpTabs v-model="tab" :items="tabs">
        <template #kernel>
          <KernelLogPanel :initial-query="initialQuery" />
        </template>
        <template #events>
          <LiveEventLog />
        </template>
      </FpTabs>
    </div>
  </LayoutContent>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'

import LayoutContent from '@/components/ui/LayoutContent.vue'
import FpTabs from '@/components/ui/FpTabs.vue'
import KernelLogPanel from '@/components/logs/KernelLogPanel.vue'
import LiveEventLog from '@/components/logs/LiveEventLog.vue'

type LogTab = 'kernel' | 'events'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()

const tabs = computed(() => [
  { value: 'kernel', label: t('log.kernel.tab'), icon: 'oi oi-file' },
  { value: 'events', label: t('log.kernel.eventsTab'), icon: 'oi oi-bolt' },
])

/** 标签页与 `?tab=` 双向同步（刷新 / 分享链接保持位置）；默认内核日志 */
const tab = computed<string>({
  get: () => (route.query.tab === 'events' ? 'events' : 'kernel'),
  set: (v) => {
    const next: LogTab = v === 'events' ? 'events' : 'kernel'
    void router.replace({ query: { ...route.query, tab: next === 'kernel' ? undefined : next } })
  },
})

/** 深链：错误提示中的请求编号 → `/logs?q=<id>` 直接筛选 */
const initialQuery = computed(() => (typeof route.query.q === 'string' ? route.query.q : undefined))
</script>
