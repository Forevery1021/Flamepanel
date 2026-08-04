<template>
  <div class="view-container">
    <el-card shadow="hover">
      <el-tabs v-model="tab">
        <el-tab-pane :label="t('memo.memos')" name="memo">
          <MemoList :key="'memo'" kind="memo" />
        </el-tab-pane>
        <el-tab-pane :label="t('memo.todos')" name="todo">
          <MemoList :key="'todo'" kind="todo" />
        </el-tab-pane>
      </el-tabs>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import MemoList from './MemoList.vue'

const { t } = useI18n()
const tab = ref('memo')
</script>
