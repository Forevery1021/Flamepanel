<template>
  <div class="fp-tabs">
    <Tabs v-bind="$attrs" :value="model" @update:value="onChange">
      <TabList>
        <Tab v-for="item in items" :key="item.value" :value="item.value">
          <i v-if="item.icon" :class="item.icon" class="fp-tabs__icon" />
          {{ item.label }}
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
.fp-tabs {
  width: 100%;
}
.fp-tabs__icon {
  margin-right: 6px;
}
</style>
