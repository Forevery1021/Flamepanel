<template>
  <LayoutContent :title="t('memo.memos')">
    <div class="panel">
      <Tabs v-model:value="tab">
        <TabList>
          <Tab value="memo">{{ t('memo.memos') }}</Tab>
          <Tab value="todo">{{ t('memo.todos') }}</Tab>
        </TabList>
        <TabPanels>
          <TabPanel value="memo">
            <MemoList :key="'memo'" kind="memo" />
          </TabPanel>
          <TabPanel value="todo">
            <MemoList :key="'todo'" kind="todo" />
          </TabPanel>
        </TabPanels>
      </Tabs>
    </div>
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import Tabs from 'openvue/tabs'
import TabList from 'openvue/tablist'
import Tab from 'openvue/tab'
import TabPanels from 'openvue/tabpanels'
import TabPanel from 'openvue/tabpanel'
import MemoList from './MemoList.vue'
import LayoutContent from '@/components/ui/LayoutContent.vue'

const { t } = useI18n()
const tab = ref('memo')
</script>

<style scoped>
.panel {
  padding: var(--fp-space-4);
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
}
</style>
