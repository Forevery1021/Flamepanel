<template>
  <div class="cap-tags">
    <template v-if="views.length">
      <FpTag
        v-for="c in views"
        :key="c.raw"
        :severity="c.severity"
        :value="detailed ? t(c.key, c.params) : c.raw"
        :title="detailed ? c.raw : t(c.key, c.params)"
      />
    </template>
    <span v-else class="cap-none">{{ t('plugin.noCapabilities') }}</span>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import FpTag from '@/components/ui/FpTag.vue'
import { describeCapability } from '@/utils/pluginCapabilities'

/** 宿主能力徽标：fs:write 红 / fs:read 黄 / 其余蓝；detailed 时显示人话描述 */
const props = withDefaults(defineProps<{ capabilities?: string[]; detailed?: boolean }>(), {
  capabilities: () => [],
  detailed: false,
})
const { t } = useI18n()
const views = computed(() => props.capabilities.map(describeCapability))
</script>

<style scoped>
.cap-tags {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fp-space-1);
}
.cap-none {
  font-size: 12px;
  color: var(--fp-text-muted);
}
</style>
