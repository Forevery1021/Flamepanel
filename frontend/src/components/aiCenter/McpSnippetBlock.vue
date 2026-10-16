<template>
  <div class="mcp-snippets">
    <FpSelectButton
      v-model="client"
      :options="clientOptions"
      option-label="label"
      option-value="value"
      :allow-empty="false"
      :aria-label="t('aiCenter.client')"
    />
    <p class="where">{{ t(`aiCenter.clients.${current.client}.where`) }}</p>
    <div class="code-wrap">
      <pre class="code"><code>{{ current.code }}</code></pre>
      <CopyButton class="copy" :text="current.code" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import FpSelectButton from '@/components/ui/FpSelectButton.vue'
import CopyButton from '@/components/ui/CopyButton.vue'
import { buildMcpSnippets, TOKEN_PLACEHOLDER, type McpClientId } from '@/utils/mcpSnippets'

const props = withDefaults(defineProps<{ url: string; token?: string }>(), { token: TOKEN_PLACEHOLDER })
const { t } = useI18n()

const client = ref<string>('cursor')
const snippets = computed(() => buildMcpSnippets(props.url, props.token))
const current = computed(() => snippets.value.find((s) => s.client === client.value) ?? snippets.value[0]!)

const clientOptions = computed(() =>
  snippets.value.map((s) => ({ label: t(`aiCenter.clients.${s.client as McpClientId}.label`), value: s.client })),
)
</script>

<style scoped>
.mcp-snippets {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-2);
}
.where {
  margin: 0;
  color: var(--fp-text-muted);
  font-size: 13px;
}
.code-wrap {
  position: relative;
}
.code {
  margin: 0;
  padding: var(--fp-space-3);
  padding-right: 44px;
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius, 6px);
  background: var(--fp-bg-subtle, transparent);
  font-size: 12.5px;
  line-height: 1.5;
  overflow-x: auto;
  white-space: pre;
}
.copy {
  position: absolute;
  top: 6px;
  right: 6px;
}
</style>
