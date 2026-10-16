<template>
  <div class="panel">
    <div class="panel-header">
      <span class="panel-title">{{ t('dashboard.processTop') }}</span>
    </div>
    <FpTable :rows="rows" :paginator="false" size="small" :empty-text="t('common.noData')" class="process-table">
      <FpColumn field="name" :header="t('dashboard.processName')" />
      <FpColumn field="cpu" :header="t('dashboard.processCpu')" style="width: 80px">
        <template #body="{ data }">
          <span class="mono num" :class="{ hot: Number(data.cpu) >= 50 }">{{ Number(data.cpu).toFixed(1) }}%</span>
        </template>
      </FpColumn>
      <FpColumn field="memory_mb" :header="t('dashboard.processMem')" style="width: 90px">
        <template #body="{ data }">
          <span class="mono num">{{ data.memory_mb }} MB</span>
        </template>
      </FpColumn>
      <FpColumn field="pid" :header="t('dashboard.processPid')" style="width: 70px">
        <template #body="{ data }">
          <span class="mono">{{ data.pid }}</span>
        </template>
      </FpColumn>
    </FpTable>
  </div>
</template>

<script setup lang="ts">
// 样式见文末：进程表用等宽数字右对齐，CPU 列按占用着色
import { useI18n } from 'vue-i18n'
import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import type { ProcessEntry } from '@/api/generated'

/**
 * 进程 TOP 面板（Doc/23 §3.3 T2-4 视图拆分第五站）。
 *
 * 四列的数据都有各自的处理口径，拆出后一并固化：
 * - 名称原样（进程名是标识符，不翻译）；
 * - CPU 保留 **1 位小数**并补 `%`（后端给的是 float，`0.30000000000000004` 这类
 *   浮点和会直出成噪声；`Number(...)` 兜住后端把数字序列化成字符串的形态）；
 * - 内存带 `MB` 单位（不做自动换算——运维比大小用的是 MB，换算成 GB 反而要心算）；
 * - PID 原样等宽。
 *
 * 性能提示：进程表**不传 `loading`**（原实现即如此）。10s 轮询下每次刷新都切
 * loading 态会让表格闪烁成骨架屏；保持旧数据直到新数据到达，观感是「就地更新」。
 */
defineProps<{ rows: ProcessEntry[] }>()

const { t } = useI18n()
</script>

<style scoped>
.mono {
  font-family: var(--fp-font-mono);
  font-size: 12px;
}
</style>
