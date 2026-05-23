<script setup lang="ts">
import { ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import api from '@/api/client'
import type { CleanupItem, CleanupScanResult, CleanupResult } from '@/types'

const scanning = ref(false)
const cleaning = ref(false)
const items = ref<CleanupItem[]>([])
const totalDisplay = ref('')
const selectedCategories = ref<string[]>([])
const result = ref<CleanupResult | null>(null)

const categoryMap: Record<string, { label: string; icon: string }> = {
  temp: { label: '系统临时文件', icon: '🧹' },
  docker: { label: 'Docker 缓存', icon: '🐳' },
  package: { label: '包管理器缓存', icon: '📦' },
  logs: { label: '旧日志文件', icon: '📋' },
  dev: { label: '开发构建产物', icon: '🔧' },
}

const groupedItems = () => {
  const map: Record<string, CleanupItem[]> = {}
  for (const item of items.value) {
    if (!map[item.category]) map[item.category] = []
    map[item.category].push(item)
  }
  return map
}

const handleScan = async () => {
  scanning.value = true
  result.value = null
  try {
    const res = await api.get<CleanupScanResult>('/cleanup/scan')
    items.value = res.data.items
    totalDisplay.value = res.data.total_display
    selectedCategories.value = [...new Set(items.value.map(i => i.category))]
    if (items.value.length === 0) {
      ElMessage.info('没有发现可清理的文件')
    } else {
      ElMessage.success(`发现 ${items.value.length} 项可清理内容`)
    }
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '扫描失败')
  } finally {
    scanning.value = false
  }
}

const toggleCategory = (cat: string) => {
  const idx = selectedCategories.value.indexOf(cat)
  if (idx >= 0) {
    selectedCategories.value.splice(idx, 1)
  } else {
    selectedCategories.value.push(cat)
  }
}

const handleClean = async () => {
  if (selectedCategories.value.length === 0) {
    ElMessage.warning('请至少选择一个类别')
    return
  }
  try {
    await ElMessageBox.confirm(
      `确定要清理选中的 ${selectedCategories.value.length} 个类别吗？`,
      '确认清理',
      { type: 'warning' }
    )
  } catch {
    return
  }
  cleaning.value = true
  try {
    const res = await api.post<CleanupResult>('/cleanup/run', {
      categories: selectedCategories.value,
    })
    result.value = res.data
    if (res.data.errors.length > 0) {
      ElMessage.warning(`清理完成，但有 ${res.data.errors.length} 个错误`)
    } else {
      ElMessage.success(`清理完成，释放了 ${res.data.freed_display} 空间`)
    }
    // Refresh scan
    await handleScan()
  } catch (e: any) {
    ElMessage.error(e.response?.data?.message || '清理失败')
  } finally {
    cleaning.value = false
  }
}

const selectAll = () => {
  if (selectedCategories.value.length === Object.keys(groupedItems()).length) {
    selectedCategories.value = []
  } else {
    selectedCategories.value = Object.keys(groupedItems())
  }
}
</script>

<template>
  <div class="cleanup-page">
    <div class="page-header">
      <h2>系统清理</h2>
      <p class="desc">扫描并清理系统中的缓存、日志和垃圾文件，释放磁盘空间</p>
    </div>

    <div class="toolbar">
      <el-button type="primary" :loading="scanning" :icon="'Search'" @click="handleScan">
        {{ scanning ? '扫描中...' : '扫描系统' }}
      </el-button>
      <el-button
        type="danger"
        :disabled="selectedCategories.length === 0"
        :loading="cleaning"
        @click="handleClean"
      >
        {{ cleaning ? '清理中...' : '清理选中项' }}
      </el-button>
      <span v-if="totalDisplay" class="total-hint">
        共发现 <strong>{{ totalDisplay }}</strong> 可释放空间
      </span>
    </div>

    <!-- Scan results -->
    <div v-if="items.length > 0" class="scan-results">
      <div class="category-header">
        <el-checkbox
          :model-value="selectedCategories.length === Object.keys(groupedItems()).length"
          :indeterminate="selectedCategories.length > 0 && selectedCategories.length < Object.keys(groupedItems()).length"
          @change="selectAll"
        >
          全选 / 取消
        </el-checkbox>
      </div>

      <div
        v-for="(groupItems, category) in groupedItems()"
        :key="category"
        class="category-card"
        :class="{ selected: selectedCategories.includes(category) }"
      >
        <div class="category-head" @click="toggleCategory(category)">
          <el-checkbox
            :model-value="selectedCategories.includes(category)"
            @change="toggleCategory(category)"
            @click.stop
          />
          <span class="cat-icon">{{ categoryMap[category]?.icon || '📁' }}</span>
          <span class="cat-name">{{ categoryMap[category]?.label || category }}</span>
          <el-tag size="small">{{ groupItems.length }} 项</el-tag>
        </div>
        <div class="category-items">
          <div v-for="item in groupItems" :key="item.name" class="cleanup-item">
            <div class="item-info">
              <span class="item-name">{{ item.name }}</span>
              <span class="item-desc">{{ item.description }}</span>
              <span class="item-path">{{ item.path }}</span>
            </div>
            <div class="item-size">
              <el-tag :type="item.size_bytes > 0 ? 'warning' : 'info'" size="small">
                {{ item.size_display }}
              </el-tag>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Empty state -->
    <el-empty
      v-if="!scanning && items.length === 0 && !result"
      description="点击「扫描系统」发现可清理内容"
    />

    <!-- Cleanup result -->
    <div v-if="result" class="clean-result">
      <el-alert
        v-if="result.errors.length === 0"
        title="清理完成"
        type="success"
        :description="`成功清理了 ${result.cleaned_items.length} 项，释放 ${result.freed_display} 空间`"
        show-icon
        closable
      />
      <el-alert
        v-else
        title="清理完成（有错误）"
        type="warning"
        show-icon
        closable
      >
        <template #default>
          <p>成功: {{ result.cleaned_items.join('、') }}</p>
          <p>释放空间: {{ result.freed_display }}</p>
          <p v-if="result.errors.length > 0" style="color: #f56c6c">
            错误: {{ result.errors.join('; ') }}
          </p>
        </template>
      </el-alert>
    </div>
  </div>
</template>

<style scoped>
.cleanup-page {
  padding: 24px;
  max-width: 960px;
}

.page-header h2 {
  margin: 0;
  font-size: 22px;
  color: #303133;
}
.page-header .desc {
  margin: 4px 0 0;
  color: #909399;
  font-size: 13px;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin: 20px 0;
}

.total-hint {
  color: #67c23a;
  font-size: 14px;
  margin-left: 12px;
}
.total-hint strong {
  color: #e6a23c;
}

.scan-results {
  margin-top: 16px;
}

.category-header {
  margin-bottom: 12px;
}

.category-card {
  border: 1px solid #e4e7ed;
  border-radius: 8px;
  margin-bottom: 12px;
  overflow: hidden;
  transition: border-color 0.2s;
}
.category-card.selected {
  border-color: #409eff;
}

.category-head {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  background: #f5f7fa;
  cursor: pointer;
  user-select: none;
}
.cat-icon {
  font-size: 18px;
}
.cat-name {
  font-weight: 600;
  color: #303133;
  flex: 1;
}

.category-items {
  padding: 8px 16px 12px;
}

.cleanup-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px solid #f0f0f0;
}
.cleanup-item:last-child {
  border-bottom: none;
}

.item-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.item-name {
  font-size: 14px;
  color: #303133;
}
.item-desc {
  font-size: 12px;
  color: #909399;
}
.item-path {
  font-size: 11px;
  color: #c0c4cc;
  font-family: monospace;
}

.clean-result {
  margin-top: 20px;
}
</style>
