<template>
  <div class="market">
    <FpInlineMessage v-if="remoteError" severity="warn">
      {{ t('plugin.remoteError', { msg: remoteError }) }}
    </FpInlineMessage>
    <FpStatePanel
      :loading="loading && !items.length"
      :error="loadError"
      :empty="!items.length"
      retryable
      @retry="query.refresh()"
    >
      <div class="market-grid">
        <FpCard v-for="p in items" :key="p.id">
          <div class="card-head">
            <div class="card-title">
              <span class="name">{{ p.name }}</span>
              <span class="ver">v{{ p.version }}</span>
            </div>
            <FpTag
              :severity="p.source === 'builtin' ? 'neutral' : 'info'"
              :value="t(`plugin.source.${p.source}`)"
            />
          </div>
          <div class="card-id mono">{{ p.id }}</div>
          <p class="card-desc">{{ p.description }}</p>
          <div class="card-caps">
            <span class="caps-label">{{ t('plugin.capabilities') }}</span>
            <CapabilityTags :capabilities="p.capabilities" />
          </div>
          <div class="card-foot">
            <span class="author">{{ p.author }}</span>
            <FpTag v-if="p.installed" severity="success" :value="t('plugin.installed')" />
            <FpButton
              v-else
              v-permission="{ perm: 'plugin:create', mode: 'view' }"
              variant="primary"
              icon="oi oi-cloud-download"
              :loading="installing === p.id"
              @click="requestInstall(p)"
            >
              {{ t('plugin.install') }}
            </FpButton>
          </div>
        </FpCard>
      </div>
    </FpStatePanel>

    <FpModal v-model="consentOpen" :header="t('plugin.consentTitle')" size="md">
      <template v-if="consentFor">
        <p class="consent-intro">{{ t('plugin.consentIntro', { name: consentFor.name }) }}</p>
        <CapabilityTags :capabilities="consentFor.capabilities" detailed />
        <div class="consent-sha mono">sha256: {{ consentFor.wasm_sha256 }}</div>
      </template>
      <template #footer>
        <FpButton variant="ghost" @click="consentOpen = false">{{ t('common.cancel') }}</FpButton>
        <FpButton variant="danger" @click="acceptConsent">{{ t('plugin.consentAccept') }}</FpButton>
      </template>
    </FpModal>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import FpCard from '@/components/ui/FpCard.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpStatePanel from '@/components/ui/FpStatePanel.vue'
import FpInlineMessage from '@/components/ui/FpInlineMessage.vue'
import { useFpToast } from '@/components/ui/FpToast'
import CapabilityTags from './CapabilityTags.vue'
import { installMarketPlugin, listMarketPlugins } from '@/api/plugins'
import { useApiQuery, useQueryCacheClient } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import { getErrorMessage } from '@/utils/error'
import type { MarketListResponse, MarketPlugin } from '@/types'

/** 插件市场：内置 + 远程目录；申请能力的插件安装前必须经授权确认弹窗 */
const emit = defineEmits<{ installed: [id: string] }>()
const { t } = useI18n()
const toast = useFpToast()
const queryClient = useQueryCacheClient()

const query = useApiQuery<MarketListResponse>(
  () => queryKeys.plugins.market(),
  async () => {
    const res = await listMarketPlugins()
    return { data: res.data }
  },
)
const items = computed(() => query.data.value?.plugins ?? [])
const remoteError = computed(() => query.data.value?.remote_error ?? '')
const loading = query.loading
const loadError = query.error

const installing = ref('')
const consentOpen = ref(false)
const consentFor = ref<MarketPlugin | null>(null)

function requestInstall(p: MarketPlugin) {
  if (p.capabilities.length) {
    consentFor.value = p
    consentOpen.value = true
  } else {
    void install(p, false)
  }
}

function acceptConsent() {
  const p = consentFor.value
  consentOpen.value = false
  if (p) void install(p, true)
}

async function install(p: MarketPlugin, accept: boolean) {
  installing.value = p.id
  try {
    await installMarketPlugin(p.id, accept)
    toast.success(t('common.success'))
    queryClient.invalidateQueries({ queryKey: queryKeys.plugins.all })
    emit('installed', p.id)
  } catch (e: unknown) {
    toast.error(getErrorMessage(e, t('common.failed')))
  } finally {
    installing.value = ''
  }
}
</script>

<style scoped>
.market {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
}
.market-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: var(--fp-space-3);
}
.card-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--fp-space-2);
}
.card-title .name {
  font-weight: 600;
  color: var(--fp-text-primary);
}
.card-title .ver,
.card-id,
.author,
.caps-label {
  font-size: 12px;
  color: var(--fp-text-muted);
}
.card-title .ver {
  margin-left: var(--fp-space-2);
}
.card-desc {
  margin: var(--fp-space-2) 0;
  font-size: 13px;
  color: var(--fp-text-secondary);
  min-height: 2.6em;
}
.card-caps {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-1);
  margin-bottom: var(--fp-space-3);
}
.card-foot {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.consent-intro {
  margin: 0 0 var(--fp-space-3);
  color: var(--fp-text-primary);
}
.consent-sha {
  margin-top: var(--fp-space-3);
  font-size: 11px;
  color: var(--fp-text-muted);
  word-break: break-all;
}
</style>
