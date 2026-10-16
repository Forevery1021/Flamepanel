<template>
  <div class="fp-tabs">
    <Tabs v-bind="$attrs" :value="model" @update:value="onChange">
      <TabList>
        <Tab v-for="item in items" :key="item.value" :value="item.value">
          <i v-if="item.icon" :class="item.icon" class="fp-tabs__icon" />
          {{ item.label }}
          <span v-if="item.badge" class="fp-tabs__badge">{{ item.badge }}</span>
        </Tab>
      </TabList>
      <TabPanels>
        <TabPanel v-for="item in items" :key="item.value" :value="item.value">
          <slot :name="item.value" />
        </TabPanel>
      </TabPanels>
    </Tabs>
  </div>
</template>

<script setup lang="ts">
import Tabs from 'openvue/tabs'
import TabList from 'openvue/tablist'
import Tab from 'openvue/tab'
import TabPanels from 'openvue/tabpanels'
import TabPanel from 'openvue/tabpanel'

defineOptions({ inheritAttrs: false })

export interface FpTabItem {
  value: string
  label: string
  icon?: string
  /** 页签角标（如待处理数）；0 / 空不显示 */
  badge?: number | string
}

withDefaults(
  defineProps<{
    items?: FpTabItem[]
  }>(),
  { items: () => [] },
)

const model = defineModel<string>({ default: '' })

function onChange(v: string | number | undefined) {
  model.value = String(v ?? '')
}
</script>

<style scoped>
.fp-tabs__badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 1.25rem;
  height: 1.25rem;
  padding: 0 0.375rem;
  margin-left: 0.375rem;
  border-radius: 999px;
  background: var(--fp-color-danger, #dc2626);
  color: #fff;
  font-size: 0.75rem;
  font-weight: 600;
  line-height: 1;
}
.fp-tabs {
  width: 100%;
}
.fp-tabs__icon {
  margin-right: 6px;
}
</style>
