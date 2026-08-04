<template>
  <div class="memo-list">
    <div class="memo-toolbar">
      <el-input
        v-model="newContent"
        :placeholder="t('memo.placeholder')"
        size="small"
        class="memo-input"
        @keyup.enter="add"
      />
      <el-button type="primary" size="small" :loading="adding" @click="add">{{
        t('common.add')
      }}</el-button>
      <el-checkbox v-model="showDone" size="small">{{ t('memo.showDone') }}</el-checkbox>
    </div>

    <div v-loading="loading" class="memo-items">
      <div v-for="m in items" :key="m.id" class="memo-item" :class="{ done: m.done }">
        <el-checkbox
          :model-value="m.done"
          @change="(val: string | number | boolean) => toggle(m, Boolean(val))"
        />
        <span class="memo-content" @dblclick="startEdit(m)">{{ m.content }}</span>
        <span class="memo-time">{{ shortTime(m.created_at) }}</span>
        <el-button text size="small" @click="startEdit(m)">
          <el-icon><Edit /></el-icon>
        </el-button>
        <el-popconfirm :title="t('memo.deleteConfirm')" @confirm="remove(m.id)">
          <template #reference>
            <el-button text size="small" type="danger">
              <el-icon><Delete /></el-icon>
            </el-button>
          </template>
        </el-popconfirm>
      </div>
      <div v-if="!items.length && !loading" class="memo-empty">{{ t('common.noData') }}</div>
    </div>

    <el-dialog v-model="editVisible" :title="t('memo.edit')" width="480px">
      <el-input v-model="editContent" type="textarea" :rows="4" />
      <template #footer>
        <el-button @click="editVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" @click="saveEdit">{{ t('common.save') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { listMemos, createMemo, updateMemo, deleteMemo } from '@/api/memos'
import { ElMessage } from 'element-plus'
import { Edit, Delete } from '@element-plus/icons-vue'
import type { Memo } from '@/types'

const props = defineProps<{ kind: string }>()
const { t } = useI18n()

const items = ref<Memo[]>([])
const loading = ref(false)
const adding = ref(false)
const newContent = ref('')
const showDone = ref(true)

const editVisible = ref(false)
const editContent = ref('')
let editingId = 0

async function fetch() {
  loading.value = true
  try {
    const res = await listMemos(props.kind, showDone.value ? undefined : false)
    items.value = res.data
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    loading.value = false
  }
}

async function add() {
  if (!newContent.value.trim()) return
  adding.value = true
  try {
    await createMemo(newContent.value.trim(), props.kind)
    newContent.value = ''
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  } finally {
    adding.value = false
  }
}

async function toggle(m: Memo, done: boolean) {
  try {
    await updateMemo(m.id, { done })
    m.done = done
  } catch {
    ElMessage.error(t('common.failed'))
  }
}

function startEdit(m: Memo) {
  editingId = m.id
  editContent.value = m.content
  editVisible.value = true
}

async function saveEdit() {
  if (!editContent.value.trim()) return
  try {
    await updateMemo(editingId, { content: editContent.value.trim() })
    editVisible.value = false
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}

async function remove(id: number) {
  try {
    await deleteMemo(id)
    await fetch()
  } catch {
    ElMessage.error(t('common.failed'))
  }
}

function shortTime(ts: string) {
  const d = new Date(ts)
  return `${d.getMonth() + 1}/${d.getDate()} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
}

watch(showDone, fetch)
onMounted(fetch)
</script>

<style scoped>
.memo-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}
.memo-input {
  max-width: 420px;
}
.memo-items {
  min-height: 200px;
}
.memo-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  border-bottom: 1px solid var(--border-color);
}
.memo-item:hover {
  background: var(--bg-hover);
}
.memo-item.done .memo-content {
  text-decoration: line-through;
  color: var(--text-muted);
}
.memo-content {
  flex: 1;
  font-size: 14px;
  color: var(--text-primary);
  cursor: text;
  word-break: break-all;
}
.memo-time {
  font-size: 12px;
  color: var(--text-muted);
  flex-shrink: 0;
}
.memo-empty {
  padding: 40px;
  text-align: center;
  color: var(--text-muted);
}
</style>
