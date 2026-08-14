<template>
  <LayoutContent :title="t('memo.memos')">
    <div class="panel">
      <FpTabs v-model="tab" :items="tabItems">
<template #memo>
<MemoList :key="'memo'" kind="memo" />
</template>
<template #todo>
<MemoList :key="'todo'" kind="todo" />
</template>
</FpTabs>
    </div>
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import MemoList from './MemoList.vue'
import FpTabs from '@/components/ui/FpTabs.vue'
import type { FpTabItem } from '@/components/ui/FpTabs.vue'
import LayoutContent from '@/components/ui/LayoutContent.vue'

const { t } = useI18n()

const tabItems: FpTabItem[] = [
  { value: 'memo', label: t('memo.memos') },
  { value: 'todo', label: t('memo.todos') },
]
const tab = ref('memo')
</script>

<style scoped>
</style>
